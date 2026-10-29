//! Optional desktop effects. The renderer always has a readable opaque fallback.
use crate::appearance::ThemePreset;

#[derive(Default)]
pub struct PlatformEffects {
    #[cfg(target_os = "linux")]
    wayland: Option<linux::Effects>,
    #[cfg(target_os = "windows")]
    last_theme: Option<ThemePreset>,
    active: bool,
}

impl PlatformEffects {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        #[cfg(target_os = "linux")]
        return Self {
            wayland: linux::Effects::new(cc).ok(),
            active: false,
        };
        #[cfg(not(target_os = "linux"))]
        {
            let _ = cc;
            Self::default()
        }
    }

    pub fn update(
        &mut self,
        frame: &eframe::Frame,
        theme: ThemePreset,
        size: eframe::egui::Vec2,
    ) -> bool {
        #[cfg(target_os = "linux")]
        {
            let _ = frame;
            self.active = self
                .wayland
                .as_mut()
                .is_some_and(|effects| effects.update(theme.glass(), size));
        }
        #[cfg(target_os = "windows")]
        if self.last_theme != Some(theme) {
            self.active = windows::apply(frame, theme);
            self.last_theme = Some(theme);
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        let _ = (frame, theme, size);
        #[cfg(target_os = "windows")]
        let _ = size;
        self.active
    }

    pub fn active(&self) -> bool {
        self.active
    }

    /// Called by App::on_exit while the borrowed native window is still alive.
    pub fn shutdown(&mut self) {
        #[cfg(target_os = "linux")]
        {
            self.wayland.take();
        }
    }
}

#[cfg(target_os = "windows")]
mod windows {
    use super::*;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::Graphics::Dwm::{
        DwmExtendFrameIntoClientArea, DwmSetWindowAttribute, DWMWA_SYSTEMBACKDROP_TYPE,
        DWMWA_USE_IMMERSIVE_DARK_MODE,
    };
    use windows_sys::Win32::UI::Controls::MARGINS;

    pub fn apply(frame: &eframe::Frame, theme: ThemePreset) -> bool {
        let Ok(handle) = frame.window_handle() else {
            return false;
        };
        let RawWindowHandle::Win32(handle) = handle.as_raw() else {
            return false;
        };
        let hwnd = handle.hwnd.get() as *mut std::ffi::c_void;
        let dark: i32 = theme.dark().into();
        let backdrop: i32 = if theme.glass() { 3 } else { 1 }; // Desktop Acrylic / none.
        let margins = MARGINS {
            cxLeftWidth: -1,
            cxRightWidth: -1,
            cyTopHeight: -1,
            cyBottomHeight: -1,
        };
        // SAFETY: eframe owns this live HWND for the duration of the call. DWM copies
        // the typed attribute values; no pointers escape into application state.
        unsafe {
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_USE_IMMERSIVE_DARK_MODE as u32,
                (&dark as *const i32).cast(),
                4,
            );
            let result = DwmSetWindowAttribute(
                hwnd,
                DWMWA_SYSTEMBACKDROP_TYPE as u32,
                (&backdrop as *const i32).cast(),
                4,
            );
            if theme.glass() && result >= 0 {
                return DwmExtendFrameIntoClientArea(hwnd, &margins) >= 0;
            }
        }
        false
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};
    use wayland_client::globals::{registry_queue_init, GlobalListContents};
    use wayland_client::protocol::{
        wl_compositor::WlCompositor, wl_region::WlRegion, wl_registry, wl_surface::WlSurface,
    };
    use wayland_client::{
        delegate_noop, Connection, Dispatch, EventQueue, Proxy, QueueHandle, WEnum,
    };
    use wayland_protocols::ext::background_effect::v1::client::{
        ext_background_effect_manager_v1::{self, ExtBackgroundEffectManagerV1},
        ext_background_effect_surface_v1::ExtBackgroundEffectSurfaceV1,
    };
    use wayland_protocols_plasma::blur::client::{
        org_kde_kwin_blur::OrgKdeKwinBlur, org_kde_kwin_blur_manager::OrgKdeKwinBlurManager,
    };

    #[derive(Default)]
    struct State {
        can_blur: bool,
    }

    impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for State {
        fn event(
            _: &mut Self,
            _: &wl_registry::WlRegistry,
            _: wl_registry::Event,
            _: &GlobalListContents,
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
        }
    }
    impl Dispatch<ExtBackgroundEffectManagerV1, ()> for State {
        fn event(
            state: &mut Self,
            _: &ExtBackgroundEffectManagerV1,
            event: ext_background_effect_manager_v1::Event,
            _: &(),
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
            if let ext_background_effect_manager_v1::Event::Capabilities { flags } = event {
                state.can_blur = matches!(flags, WEnum::Value(bits) if bits.contains(ext_background_effect_manager_v1::Capability::Blur));
            }
        }
    }
    delegate_noop!(State: ignore WlCompositor);
    delegate_noop!(State: ignore WlRegion);
    delegate_noop!(State: ignore ExtBackgroundEffectSurfaceV1);
    delegate_noop!(State: ignore OrgKdeKwinBlurManager);
    delegate_noop!(State: ignore OrgKdeKwinBlur);

    pub struct Effects {
        state: State,
        queue: EventQueue<State>,
        compositor: WlCompositor,
        surface: WlSurface,
        ext_manager: Option<ExtBackgroundEffectManagerV1>,
        ext: Option<ExtBackgroundEffectSurfaceV1>,
        kde_manager: Option<OrgKdeKwinBlurManager>,
        kde: Option<OrgKdeKwinBlur>,
        last: Option<(bool, i32, i32)>,
        connection: Connection,
    }

    impl Effects {
        pub fn new(cc: &eframe::CreationContext<'_>) -> Result<Self, String> {
            let RawDisplayHandle::Wayland(display) =
                cc.display_handle().map_err(|e| e.to_string())?.as_raw()
            else {
                return Err("Not a Wayland window".into());
            };
            let RawWindowHandle::Wayland(window) =
                cc.window_handle().map_err(|e| e.to_string())?.as_raw()
            else {
                return Err("Not a Wayland surface".into());
            };
            // SAFETY: both handles are borrowed from eframe's live window. Guest
            // mode never closes its display. shutdown drops us before window teardown.
            let backend = unsafe {
                wayland_backend::client::Backend::from_foreign_display(
                    display.display.as_ptr().cast(),
                )
            };
            let connection = Connection::from_backend(backend);
            // SAFETY: import this surface solely to send extension requests. We
            // never install its event handler, commit, or destroy the borrowed surface.
            let id = unsafe {
                wayland_backend::client::ObjectId::from_ptr(
                    WlSurface::interface(),
                    window.surface.as_ptr().cast(),
                )
            }
            .map_err(|e| e.to_string())?;
            let surface = WlSurface::from_id(&connection, id).map_err(|e| e.to_string())?;
            let (globals, mut queue) =
                registry_queue_init::<State>(&connection).map_err(|e| e.to_string())?;
            let qh = queue.handle();
            let compositor = globals
                .bind::<WlCompositor, _, _>(&qh, 1..=6, ())
                .map_err(|e| e.to_string())?;
            let ext_manager = globals
                .bind::<ExtBackgroundEffectManagerV1, _, _>(&qh, 1..=1, ())
                .ok();
            let kde_manager = if ext_manager.is_none() {
                globals
                    .bind::<OrgKdeKwinBlurManager, _, _>(&qh, 1..=1, ())
                    .ok()
            } else {
                None
            };
            // Registry discovery is complete; capability updates arrive on the
            // bound effect manager, so this owned registry can be released now.
            globals.destroy();
            if ext_manager.is_none() && kde_manager.is_none() {
                return Err("Compositor has no blur protocol".into());
            }
            let mut state = State::default();
            queue.roundtrip(&mut state).map_err(|e| e.to_string())?;
            let ext = ext_manager
                .as_ref()
                .map(|m| m.get_background_effect(&surface, &qh, ()));
            Ok(Self {
                state,
                queue,
                compositor,
                surface,
                ext_manager,
                ext,
                kde_manager,
                kde: None,
                last: None,
                connection,
            })
        }

        pub fn update(&mut self, glass: bool, size: eframe::egui::Vec2) -> bool {
            if self.queue.dispatch_pending(&mut self.state).is_err() {
                return false;
            }
            let enabled =
                glass && (self.ext.is_some() && self.state.can_blur || self.kde_manager.is_some());
            let signature = (
                enabled,
                size.x.ceil().max(1.0) as i32,
                size.y.ceil().max(1.0) as i32,
            );
            if self.last == Some(signature) {
                return enabled;
            }
            self.last = Some(signature);
            let region = self.compositor.create_region(&self.queue.handle(), ());
            region.add(0, 0, signature.1, signature.2);
            if let Some(ext) = &self.ext {
                ext.set_blur_region(if enabled { Some(&region) } else { None });
            }
            if let Some(manager) = &self.kde_manager {
                if enabled {
                    let blur = self.kde.get_or_insert_with(|| {
                        manager.create(&self.surface, &self.queue.handle(), ())
                    });
                    blur.set_region(Some(&region));
                    blur.commit();
                } else if let Some(blur) = self.kde.take() {
                    manager.unset(&self.surface);
                    blur.release();
                }
            }
            region.destroy();
            self.connection.flush().is_ok() && enabled
        }
    }

    impl Drop for Effects {
        fn drop(&mut self) {
            if let Some(ext) = self.ext.take() {
                ext.destroy();
            }
            if let Some(manager) = self.ext_manager.take() {
                manager.destroy();
            }
            if let Some(blur) = self.kde.take() {
                if let Some(manager) = &self.kde_manager {
                    manager.unset(&self.surface);
                }
                blur.release();
            }
            // The KDE manager and compositor have no destructor requests.
            let _ = self.connection.flush();
        }
    }
}
