use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Language {
    #[default]
    English,
    Spanish,
}

#[allow(dead_code)]
impl Language {
    pub fn code(&self) -> &'static str {
        match self {
            Language::English => "EN",
            Language::Spanish => "ES",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Language::English => "English",
            Language::Spanish => "Español",
        }
    }

    // Top Header & Navigation
    pub fn app_title(&self) -> &'static str {
        "Valheim Modpack Studio"
    }

    pub fn btn_open_output(&self) -> &'static str {
        match self {
            Language::English => "Open Output Folder",
            Language::Spanish => "Abrir Carpeta de Salida",
        }
    }

    pub fn btn_build_quick(&self) -> &'static str {
        match self {
            Language::English => "Build Modpack ZIP",
            Language::Spanish => "Crear Modpack ZIP",
        }
    }

    pub fn tab_profile(&self) -> &'static str {
        match self {
            Language::English => "Profile & Info",
            Language::Spanish => "Perfil e Info",
        }
    }

    pub fn tab_mods(&self) -> &'static str {
        "Mods"
    }

    pub fn tab_configs(&self) -> &'static str {
        match self {
            Language::English => "Configs",
            Language::Spanish => "Configuración",
        }
    }

    pub fn tab_build(&self) -> &'static str {
        match self {
            Language::English => "Build & Export",
            Language::Spanish => "Compilar y Exportar",
        }
    }

    pub fn tab_server_sync(&self) -> &'static str {
        match self {
            Language::English => "Server Sync",
            Language::Spanish => "Sincronización Servidor",
        }
    }

    pub fn tab_help(&self) -> &'static str {
        match self {
            Language::English => "Instructions",
            Language::Spanish => "Instrucciones",
        }
    }

    pub fn status_summary(
        &self,
        source: &str,
        profile: &str,
        mods: usize,
        configs: usize,
    ) -> String {
        match self {
            Language::English => format!(
                "Source: {} | Profile: {} | {} mods | {} configs",
                source, profile, mods, configs
            ),
            Language::Spanish => format!(
                "Origen: {} | Perfil: {} | {} mods | {} configuraciones",
                source, profile, mods, configs
            ),
        }
    }

    // Profile & Info Tab
    pub fn heading_mod_manager(&self) -> &'static str {
        match self {
            Language::English => "Mod Manager & Profile Source",
            Language::Spanish => "Gestor de Mods y Origen del Perfil",
        }
    }

    pub fn label_mod_manager(&self) -> &'static str {
        match self {
            Language::English => "Mod Manager:",
            Language::Spanish => "Gestor de Mods:",
        }
    }

    pub fn label_custom_path(&self) -> &'static str {
        match self {
            Language::English => "Custom Path",
            Language::Spanish => "Ruta Personalizada",
        }
    }

    pub fn label_profiles_dir(&self) -> &'static str {
        match self {
            Language::English => "Profiles Directory:",
            Language::Spanish => "Directorio de Perfiles:",
        }
    }

    pub fn not_found(&self) -> &'static str {
        match self {
            Language::English => "Not found",
            Language::Spanish => "No encontrado",
        }
    }

    pub fn btn_browse(&self) -> &'static str {
        match self {
            Language::English => "Browse...",
            Language::Spanish => "Examinar...",
        }
    }

    pub fn label_selected_profile(&self) -> &'static str {
        match self {
            Language::English => "Selected Profile:",
            Language::Spanish => "Perfil Seleccionado:",
        }
    }

    pub fn btn_reload(&self) -> &'static str {
        match self {
            Language::English => "Reload",
            Language::Spanish => "Recargar",
        }
    }

    pub fn label_profile_stats(&self, installed: usize, enabled: usize) -> String {
        match self {
            Language::English => format!("({} mods installed, {} enabled)", installed, enabled),
            Language::Spanish => format!("({} mods instalados, {} activados)", installed, enabled),
        }
    }

    pub fn heading_metadata(&self) -> &'static str {
        match self {
            Language::English => "Modpack Metadata (Thunderstore manifest.json)",
            Language::Spanish => "Metadatos del Modpack (manifest.json de Thunderstore)",
        }
    }

    pub fn label_package_name(&self) -> &'static str {
        match self {
            Language::English => "Package Name:",
            Language::Spanish => "Nombre del Paquete:",
        }
    }

    pub fn label_version(&self) -> &'static str {
        match self {
            Language::English => "Version (SemVer):",
            Language::Spanish => "Versión (SemVer):",
        }
    }

    pub fn label_website(&self) -> &'static str {
        match self {
            Language::English => "Website URL:",
            Language::Spanish => "URL del Sitio Web:",
        }
    }

    pub fn label_description(&self) -> &'static str {
        match self {
            Language::English => "Description:",
            Language::Spanish => "Descripción:",
        }
    }

    pub fn label_icon(&self) -> &'static str {
        match self {
            Language::English => "Modpack Icon (256x256 PNG):",
            Language::Spanish => "Ícono del Modpack (PNG 256x256):",
        }
    }

    pub fn btn_remove(&self) -> &'static str {
        match self {
            Language::English => "Remove",
            Language::Spanish => "Quitar",
        }
    }

    pub fn label_default_icon(&self) -> &'static str {
        match self {
            Language::English => "(Using default embedded icon)",
            Language::Spanish => "(Usando ícono predeterminado)",
        }
    }

    pub fn heading_output_dir(&self) -> &'static str {
        match self {
            Language::English => "Output Directory",
            Language::Spanish => "Directorio de Salida",
        }
    }

    pub fn label_output_dir_desc(&self) -> &'static str {
        match self {
            Language::English => "Where the final modpack .zip and unpacked build files are saved:",
            Language::Spanish => {
                "Dónde se guardarán el .zip final del modpack y los archivos generados:"
            }
        }
    }

    pub fn btn_proceed_to_mods(&self) -> &'static str {
        match self {
            Language::English => "Proceed to Mods Selection",
            Language::Spanish => "Continuar a Selección de Mods",
        }
    }

    // Mods Tab
    pub fn heading_mods(&self) -> &'static str {
        match self {
            Language::English => "Select Mods for Modpack",
            Language::Spanish => "Seleccionar Mods para el Modpack",
        }
    }

    pub fn hint_search_mods(&self) -> &'static str {
        match self {
            Language::English => "Search mods by name or author...",
            Language::Spanish => "Buscar mods por nombre o autor...",
        }
    }

    pub fn btn_select_all(&self) -> &'static str {
        match self {
            Language::English => "Select All",
            Language::Spanish => "Seleccionar Todos",
        }
    }

    pub fn btn_deselect_all(&self) -> &'static str {
        match self {
            Language::English => "Deselect All",
            Language::Spanish => "Deseleccionar Todos",
        }
    }

    pub fn btn_select_enabled(&self) -> &'static str {
        match self {
            Language::English => "Select Enabled Only",
            Language::Spanish => "Solo Activados",
        }
    }

    pub fn label_mods_selected(&self, selected: usize, total: usize) -> String {
        match self {
            Language::English => format!("{} of {} mods selected", selected, total),
            Language::Spanish => format!("{} de {} mods seleccionados", selected, total),
        }
    }

    pub fn label_version_in_pack(&self) -> &'static str {
        match self {
            Language::English => "Version in pack:",
            Language::Spanish => "Versión en paquete:",
        }
    }

    pub fn label_disabled_in_profile(&self) -> &'static str {
        match self {
            Language::English => "(Disabled in profile)",
            Language::Spanish => "(Desactivado en el perfil)",
        }
    }

    pub fn no_mods_matching(&self) -> &'static str {
        match self {
            Language::English => "No mods match your search query.",
            Language::Spanish => "Ningún mod coincide con la búsqueda.",
        }
    }

    pub fn btn_proceed_to_configs(&self) -> &'static str {
        match self {
            Language::English => "Proceed to Configs Selection",
            Language::Spanish => "Continuar a Selección de Configuración",
        }
    }

    // Configs Tab
    pub fn heading_configs(&self) -> &'static str {
        match self {
            Language::English => "Select Configuration Files (BepInEx/config)",
            Language::Spanish => "Seleccionar Archivos de Configuración (BepInEx/config)",
        }
    }

    pub fn hint_search_configs(&self) -> &'static str {
        match self {
            Language::English => "Search configs by filename or category...",
            Language::Spanish => "Buscar configs por nombre o categoría...",
        }
    }

    pub fn label_configs_selected(&self, selected: usize, total: usize) -> String {
        match self {
            Language::English => format!("{} of {} configs selected", selected, total),
            Language::Spanish => format!("{} de {} configuraciones seleccionadas", selected, total),
        }
    }

    pub fn no_configs_found(&self) -> &'static str {
        match self {
            Language::English => {
                "No configuration files found in this profile's BepInEx/config folder."
            }
            Language::Spanish => {
                "No se encontraron archivos en la carpeta BepInEx/config de este perfil."
            }
        }
    }

    pub fn no_configs_matching(&self) -> &'static str {
        match self {
            Language::English => "No configs match your search query.",
            Language::Spanish => "Ninguna configuración coincide con la búsqueda.",
        }
    }

    pub fn btn_proceed_to_build(&self) -> &'static str {
        match self {
            Language::English => "Proceed to Build & Export",
            Language::Spanish => "Continuar a Compilar y Exportar",
        }
    }

    // Build Tab
    pub fn heading_build(&self) -> &'static str {
        match self {
            Language::English => "Build & Export Modpack",
            Language::Spanish => "Compilar y Exportar Modpack",
        }
    }

    pub fn heading_summary(&self) -> &'static str {
        match self {
            Language::English => "Modpack Summary",
            Language::Spanish => "Resumen del Modpack",
        }
    }

    pub fn label_dependencies(&self) -> &'static str {
        match self {
            Language::English => "Dependencies (manifest.json):",
            Language::Spanish => "Dependencias (manifest.json):",
        }
    }

    pub fn label_config_files(&self) -> &'static str {
        match self {
            Language::English => "Config Files (config/):",
            Language::Spanish => "Archivos de Config (config/):",
        }
    }

    pub fn label_build_output(&self) -> &'static str {
        match self {
            Language::English => "Output Destination:",
            Language::Spanish => "Destino de Salida:",
        }
    }

    pub fn btn_build_main(&self) -> &'static str {
        match self {
            Language::English => "Build Thunderstore Modpack (.zip)",
            Language::Spanish => "Crear Modpack de Thunderstore (.zip)",
        }
    }

    pub fn compiling_modpack(&self) -> &'static str {
        match self {
            Language::English => "Compiling Modpack Archive...",
            Language::Spanish => "Compilando Archivo del Modpack...",
        }
    }

    pub fn building_button_label(&self) -> &'static str {
        match self {
            Language::English => "Building...",
            Language::Spanish => "Compilando...",
        }
    }

    pub fn placeholder_no_preview(&self) -> &'static str {
        match self {
            Language::English => "Click \"Preview Changes\" to inspect pending uploads, deletions, and optional custom launcher status.",
            Language::Spanish => "Haz clic en \"Vista Previa de Cambios\" para inspeccionar subidas, eliminaciones y el estado opcional del iniciador personalizado.",
        }
    }

    pub fn heading_build_results(&self) -> &'static str {
        match self {
            Language::English => "Build Result",
            Language::Spanish => "Resultado de Compilación",
        }
    }

    pub fn label_zip_archive(&self) -> &'static str {
        match self {
            Language::English => "ZIP Archive:",
            Language::Spanish => "Archivo ZIP:",
        }
    }

    pub fn label_unpacked_folder(&self) -> &'static str {
        match self {
            Language::English => "Unpacked Build:",
            Language::Spanish => "Carpeta Descomprimida:",
        }
    }

    pub fn btn_open_folder(&self) -> &'static str {
        match self {
            Language::English => "Open Output Folder",
            Language::Spanish => "Abrir Carpeta de Salida",
        }
    }

    // Server Sync Tab - General & Connection
    pub fn sync_title(&self) -> &'static str {
        match self {
            Language::English => "Dedicated Server Sync",
            Language::Spanish => "Sincronización con Servidor Dedicado",
        }
    }

    pub fn sync_desc(&self) -> &'static str {
        match self {
            Language::English => "Keep your dedicated server's BepInEx folder in perfect sync with your local mod profile via SFTP.",
            Language::Spanish => "Mantén la carpeta BepInEx de tu servidor dedicado perfectamente sincronizada con tu perfil local mediante SFTP.",
        }
    }

    pub fn btn_dismiss(&self) -> &'static str {
        match self {
            Language::English => "Dismiss",
            Language::Spanish => "Cerrar",
        }
    }

    pub fn heading_source_profile(&self) -> &'static str {
        match self {
            Language::English => "Source Profile",
            Language::Spanish => "Perfil de Origen",
        }
    }

    pub fn source_folder_found(&self) -> &'static str {
        match self {
            Language::English => "Source folder found and ready to sync.",
            Language::Spanish => "Carpeta de origen encontrada y lista para sincronizar.",
        }
    }

    pub fn source_folder_missing(&self) -> &'static str {
        match self {
            Language::English => "Source folder not found. Select a valid profile on the Profile & Info tab.",
            Language::Spanish => "Carpeta de origen no encontrada. Selecciona un perfil válido en la pestaña Perfil e Info.",
        }
    }

    pub fn no_profile_selected(&self) -> &'static str {
        match self {
            Language::English => "No profile selected. Please select a profile on the Profile & Info tab first.",
            Language::Spanish => "No hay ningún perfil seleccionado. Selecciona un perfil en la pestaña Perfil e Info primero.",
        }
    }

    pub fn heading_connection(&self) -> &'static str {
        match self {
            Language::English => "Server Connection (SFTP)",
            Language::Spanish => "Conexión con el Servidor (SFTP)",
        }
    }

    pub fn label_host(&self) -> &'static str {
        match self {
            Language::English => "Host / IP:",
            Language::Spanish => "Servidor / IP:",
        }
    }

    pub fn label_port(&self) -> &'static str {
        match self {
            Language::English => "Port:",
            Language::Spanish => "Puerto:",
        }
    }

    pub fn label_username(&self) -> &'static str {
        match self {
            Language::English => "Username:",
            Language::Spanish => "Usuario:",
        }
    }

    pub fn label_password(&self) -> &'static str {
        match self {
            Language::English => "Password:",
            Language::Spanish => "Contraseña:",
        }
    }

    pub fn label_show_password(&self) -> &'static str {
        match self {
            Language::English => "Show",
            Language::Spanish => "Mostrar",
        }
    }

    pub fn label_staging_dir(&self) -> &'static str {
        match self {
            Language::English => "Staging Folder:",
            Language::Spanish => "Carpeta de Preparación (Staging):",
        }
    }

    pub fn chk_auto_refresh(&self) -> &'static str {
        match self {
            Language::English => "Refresh staging folder from profile before syncing",
            Language::Spanish => "Actualizar carpeta staging desde el perfil antes de sincronizar",
        }
    }

    pub fn desc_auto_refresh(&self) -> &'static str {
        match self {
            Language::English => "Mirrors BepInEx/config and BepInEx/plugins and removes obsolete files automatically.",
            Language::Spanish => "Duplica BepInEx/config y BepInEx/plugins y elimina archivos obsoletos automáticamente.",
        }
    }

    pub fn btn_save_settings(&self) -> &'static str {
        match self {
            Language::English => "Save Settings",
            Language::Spanish => "Guardar Configuración",
        }
    }

    pub fn heading_ignore(&self) -> &'static str {
        match self {
            Language::English => "Ignore Patterns (.sync_ignore)",
            Language::Spanish => "Patrones de Ignorados (.sync_ignore)",
        }
    }

    pub fn desc_ignore(&self) -> &'static str {
        match self {
            Language::English => "One glob pattern per line relative to BepInEx. Files matching these patterns won't be touched on the server.",
            Language::Spanish => "Un patrón glob por línea relativo a BepInEx. Los archivos coincidentes no serán modificados en el servidor.",
        }
    }

    pub fn btn_reset_defaults(&self) -> &'static str {
        match self {
            Language::English => "Reset to Defaults",
            Language::Spanish => "Restablecer Valores Predeterminados",
        }
    }

    pub fn btn_reload_staging(&self) -> &'static str {
        match self {
            Language::English => "Reload from Staging",
            Language::Spanish => "Recargar desde Staging",
        }
    }

    pub fn btn_save_ignore(&self) -> &'static str {
        match self {
            Language::English => "Save Ignore File",
            Language::Spanish => "Guardar Archivo de Ignorados",
        }
    }

    // Actions
    pub fn heading_actions(&self) -> &'static str {
        match self {
            Language::English => "Actions",
            Language::Spanish => "Acciones",
        }
    }

    pub fn btn_action_preview(&self) -> &'static str {
        match self {
            Language::English => "Preview Changes",
            Language::Spanish => "Vista Previa de Cambios",
        }
    }

    pub fn tip_action_preview(&self) -> &'static str {
        match self {
            Language::English => "Inspect all pending uploads and deletions safely without making any changes on the server",
            Language::Spanish => "Inspecciona todas las subidas y eliminaciones pendientes de forma segura sin realizar cambios en el servidor",
        }
    }

    pub fn btn_action_sync(&self) -> &'static str {
        match self {
            Language::English => "Sync Changes",
            Language::Spanish => "Sincronizar Cambios",
        }
    }

    pub fn tip_action_sync(&self) -> &'static str {
        match self {
            Language::English => "Upload modified and new files, and delete remote files that were removed locally",
            Language::Spanish => "Sube archivos modificados y nuevos, y elimina del servidor los archivos que ya no existan localmente",
        }
    }

    pub fn btn_action_force(&self) -> &'static str {
        match self {
            Language::English => "Force Upload All",
            Language::Spanish => "Forzar Subida Completa",
        }
    }

    pub fn tip_action_force(&self) -> &'static str {
        match self {
            Language::English => "Re-upload every local file regardless of remote checksums",
            Language::Spanish => {
                "Vuelve a subir todos los archivos locales sin importar los checksums remotos"
            }
        }
    }

    pub fn btn_action_upload_only(&self) -> &'static str {
        match self {
            Language::English => "Upload Only",
            Language::Spanish => "Solo Subir Archivos",
        }
    }

    pub fn tip_action_upload_only(&self) -> &'static str {
        match self {
            Language::English => "Upload changed files without deleting any files from the server",
            Language::Spanish => "Sube los archivos modificados sin borrar nada en el servidor",
        }
    }

    pub fn btn_action_wrapper(&self) -> &'static str {
        match self {
            Language::English => "Check & Fix Wrapper",
            Language::Spanish => "Verificar y Reparar Wrapper",
        }
    }

    pub fn tip_action_wrapper(&self) -> &'static str {
        match self {
            Language::English => "Verify and fix valheim_server wrapper script and valheim_server_Data symlink",
            Language::Spanish => "Verifica y repara el script wrapper de valheim_server y el enlace simbólico valheim_server_Data",
        }
    }

    pub fn btn_action_refresh(&self) -> &'static str {
        match self {
            Language::English => "Refresh Staging",
            Language::Spanish => "Actualizar Staging",
        }
    }

    pub fn tip_action_refresh(&self) -> &'static str {
        match self {
            Language::English => {
                "Copy plugins and config from selected profile into staging folder"
            }
            Language::Spanish => {
                "Copia plugins y configuración del perfil seleccionado a la carpeta staging"
            }
        }
    }

    pub fn btn_action_test(&self) -> &'static str {
        match self {
            Language::English => "Test Connection",
            Language::Spanish => "Probar Conexión",
        }
    }

    pub fn tip_action_test(&self) -> &'static str {
        match self {
            Language::English => "Connect to SFTP server and verify BepInEx directory access",
            Language::Spanish => {
                "Conectar al servidor SFTP y verificar el acceso a la carpeta BepInEx"
            }
        }
    }

    pub fn sync_in_progress(&self) -> &'static str {
        match self {
            Language::English => "Sync in progress...",
            Language::Spanish => "Sincronización en curso...",
        }
    }

    pub fn btn_cancel(&self) -> &'static str {
        match self {
            Language::English => "Cancel",
            Language::Spanish => "Cancelar",
        }
    }

    // Preview Section (UI/UX Redesign)
    pub fn heading_preview(&self) -> &'static str {
        match self {
            Language::English => "Sync Inspection & Preview",
            Language::Spanish => "Inspección y Vista Previa de Cambios",
        }
    }

    pub fn badge_uploads(&self) -> &'static str {
        match self {
            Language::English => "Uploads",
            Language::Spanish => "Subidas",
        }
    }

    pub fn badge_deletions(&self) -> &'static str {
        match self {
            Language::English => "Deletions",
            Language::Spanish => "Eliminaciones",
        }
    }

    pub fn badge_local(&self) -> &'static str {
        match self {
            Language::English => "Local Files",
            Language::Spanish => "Archivos Locales",
        }
    }

    pub fn badge_remote(&self) -> &'static str {
        match self {
            Language::English => "Remote Files",
            Language::Spanish => "Archivos Remotos",
        }
    }

    pub fn badge_wrapper(&self) -> &'static str {
        match self {
            Language::English => "Server Wrapper",
            Language::Spanish => "Wrapper Servidor",
        }
    }

    pub fn wrapper_intact(&self) -> &'static str {
        match self {
            Language::English => "Ready & Intact",
            Language::Spanish => "Listo e Intacto",
        }
    }

    pub fn wrapper_rename_needed(&self) -> &'static str {
        match self {
            Language::English => "Rename Needed",
            Language::Spanish => "Renombrado Necesario",
        }
    }

    pub fn wrapper_missing(&self) -> &'static str {
        match self {
            Language::English => "Missing Script",
            Language::Spanish => "Script Faltante",
        }
    }

    pub fn symlink_ok(&self) -> &'static str {
        match self {
            Language::English => "Symlink OK",
            Language::Spanish => "Enlace Simbólico OK",
        }
    }

    pub fn symlink_missing(&self) -> &'static str {
        match self {
            Language::English => "Symlink Missing",
            Language::Spanish => "Falta Enlace",
        }
    }

    pub fn tab_all_changes(&self, count: usize) -> String {
        match self {
            Language::English => format!("All Changes ({})", count),
            Language::Spanish => format!("Todos los Cambios ({})", count),
        }
    }

    pub fn tab_uploads(&self, count: usize) -> String {
        match self {
            Language::English => format!("Uploads ({})", count),
            Language::Spanish => format!("Subidas ({})", count),
        }
    }

    pub fn tab_deletions(&self, count: usize) -> String {
        match self {
            Language::English => format!("Deletions ({})", count),
            Language::Spanish => format!("Eliminaciones ({})", count),
        }
    }

    pub fn hint_search_changes(&self) -> &'static str {
        match self {
            Language::English => "Filter changes...",
            Language::Spanish => "Filtrar cambios...",
        }
    }

    pub fn msg_no_changes(&self) -> &'static str {
        match self {
            Language::English => {
                "Server is completely synchronized with local staging. No changes needed!"
            }
            Language::Spanish => {
                "¡El servidor está completamente sincronizado! No se requieren cambios."
            }
        }
    }

    pub fn msg_no_changes_filter(&self) -> &'static str {
        match self {
            Language::English => "No changed files match your filter.",
            Language::Spanish => "Ningún archivo modificado coincide con el filtro.",
        }
    }

    // Results Banner
    pub fn heading_sync_result(&self) -> &'static str {
        match self {
            Language::English => "Last Sync Result",
            Language::Spanish => "Resultado de Última Sincronización",
        }
    }

    pub fn sync_completed_title(&self, secs: f32) -> String {
        match self {
            Language::English => format!("Sync completed successfully in {:.1}s", secs),
            Language::Spanish => format!("Sincronización completada con éxito en {:.1}s", secs),
        }
    }

    pub fn sync_cancelled_title(&self) -> &'static str {
        match self {
            Language::English => "Sync was cancelled by user",
            Language::Spanish => "Sincronización cancelada por el usuario",
        }
    }

    pub fn sync_failed_title(&self) -> &'static str {
        match self {
            Language::English => "Sync completed with errors",
            Language::Spanish => "Sincronización finalizada con errores",
        }
    }

    // Logs Section (UI/UX Redesign)
    pub fn heading_logs(&self) -> &'static str {
        match self {
            Language::English => "Activity & Sync Logs",
            Language::Spanish => "Registro de Actividad y Sincronización",
        }
    }

    pub fn log_filter_all(&self, count: usize) -> String {
        match self {
            Language::English => format!("All ({})", count),
            Language::Spanish => format!("Todos ({})", count),
        }
    }

    pub fn log_filter_errors(&self, count: usize) -> String {
        match self {
            Language::English => format!("Errors ({})", count),
            Language::Spanish => format!("Errores ({})", count),
        }
    }

    pub fn log_filter_warnings(&self, count: usize) -> String {
        match self {
            Language::English => format!("Warnings ({})", count),
            Language::Spanish => format!("Alertas ({})", count),
        }
    }

    pub fn log_filter_success(&self, count: usize) -> String {
        match self {
            Language::English => format!("Success ({})", count),
            Language::Spanish => format!("Éxito ({})", count),
        }
    }

    pub fn log_filter_info(&self, count: usize) -> String {
        match self {
            Language::English => format!("Info ({})", count),
            Language::Spanish => format!("Info ({})", count),
        }
    }

    pub fn hint_search_logs(&self) -> &'static str {
        match self {
            Language::English => "Filter logs...",
            Language::Spanish => "Filtrar registros...",
        }
    }

    pub fn btn_copy_logs(&self) -> &'static str {
        match self {
            Language::English => "Copy",
            Language::Spanish => "Copiar",
        }
    }

    pub fn btn_clear_logs(&self) -> &'static str {
        match self {
            Language::English => "Clear",
            Language::Spanish => "Limpiar",
        }
    }

    pub fn chk_autoscroll(&self) -> &'static str {
        match self {
            Language::English => "Auto-scroll",
            Language::Spanish => "Desplazamiento auto",
        }
    }

    pub fn msg_logs_copied(&self) -> &'static str {
        match self {
            Language::English => "Logs copied to clipboard!",
            Language::Spanish => "¡Registros copiados al portapapeles!",
        }
    }

    pub fn no_logs_yet(&self) -> &'static str {
        match self {
            Language::English => {
                "No sync activity recorded yet. Run a preview or sync action above."
            }
            Language::Spanish => {
                "Aún no hay actividad de sincronización. Ejecuta una vista previa o acción arriba."
            }
        }
    }

    pub fn no_logs_matching(&self) -> &'static str {
        match self {
            Language::English => "No log messages match the active filter.",
            Language::Spanish => "Ningún mensaje coincide con el filtro activo.",
        }
    }

    // Instructions / Help Tab
    pub fn help_title(&self) -> &'static str {
        match self {
            Language::English => "How to Distribute Your Modpack & Sync to Server",
            Language::Spanish => "Guía de Distribución de Modpacks y Sincronización con Servidor",
        }
    }

    pub fn help_content(&self) -> &'static [(&'static str, &'static [&'static str])] {
        match self {
            Language::English => &[
                (
                    "Mod Manager Support:",
                    &[
                        "• Supports both r2modman and Gale profiles seamlessly.",
                        "• Switch between them on the 'Profile & Info' tab, or browse to any custom profile folder.",
                        "• Gale profiles use export.r2x and store configs in BepInEx/config.",
                    ],
                ),
                (
                    "Server Sync (SFTP):",
                    &[
                        "1. Select the profile you want to deploy on the 'Profile & Info' tab.",
                        "2. Open the 'Server Sync' tab and enter the server host, port, username, and password.",
                        "3. Choose a staging folder. The default is the 'valheim' folder on your Desktop.",
                        "4. Use 'Test Connection', verify the first-use SHA-256 fingerprint independently, and enter its 64 hex characters in the trusted server key field.",
                        "5. Save settings, then use 'Preview Changes' to verify uploads and deletions before syncing.",
                        "6. 'Sync Changes' uploads modified files and removes remote files that are no longer in the profile.",
                        "• 'Refresh Staging' mirrors BepInEx/config and BepInEx/plugins into the staging folder.",
                        "• Ignore patterns in .sync_ignore prevent server-side files (like admin lists or server-only mods) from being deleted.",
                        "• Leave custom BepInEx launcher checks off for provider-managed servers. Enable them under Hosting compatibility only if your host allows executable replacement.",
                        "• When launcher checks are enabled, 'Check & Fix Wrapper' restores the modded server launch script after a Valheim game update.",
                    ],
                ),
                (
                    "Option 1: Upload to Thunderstore (Public or Unlisted):",
                    &[
                        "1. Create an account on https://thunderstore.io/c/valheim/",
                        "2. Click 'Upload' in the top-right header.",
                        "3. Select your community team, attach the generated .zip from your output folder, and tag 'Modpacks'.",
                        "4. When players install it through r2modman or Gale, all mods and configs are downloaded automatically!",
                    ],
                ),
                (
                    "Option 2: Direct Share with Friends:",
                    &[
                        "• Send the generated .zip directly to your friends or extract it into their Valheim BepInEx directory.",
                    ],
                ),
            ],
            Language::Spanish => &[
                (
                    "Soporte para Gestores de Mods:",
                    &[
                        "• Compatible con perfiles de r2modman y Gale de forma totalmente integrada.",
                        "• Cambia entre ellos en la pestaña 'Perfil e Info' o selecciona cualquier carpeta personalizada.",
                        "• Los perfiles de Gale usan el formato export.r2x y guardan la configuración en BepInEx/config.",
                    ],
                ),
                (
                    "Sincronización con Servidor Dedicado (SFTP):",
                    &[
                        "1. Selecciona el perfil que deseas desplegar en la pestaña 'Perfil e Info'.",
                        "2. Entra a la pestaña 'Sincronización Servidor' e introduce el host, puerto, usuario y contraseña.",
                        "3. Elige una carpeta staging (por defecto la carpeta 'valheim' en tu Escritorio).",
                        "4. Usa 'Probar Conexión', verifica la huella SHA-256 por un medio independiente y pega sus 64 caracteres hexadecimales en el campo de clave confiable.",
                        "5. Guarda la configuración y usa 'Vista Previa de Cambios' para inspeccionar subidas y eliminaciones de forma segura.",
                        "6. 'Sincronizar Cambios' sube los archivos modificados y elimina los archivos del servidor que ya no existan en el perfil.",
                        "• 'Actualizar Staging' duplica BepInEx/config y BepInEx/plugins en la carpeta local de preparación.",
                        "• Los patrones en .sync_ignore protegen archivos del servidor (como listas de administradores o mods exclusivos del server) para que no se borren.",
                        "• Deja desactivada la revisión del iniciador BepInEx en servidores gestionados por el proveedor. Actívala en Compatibilidad del alojamiento solo si permite reemplazar el ejecutable.",
                        "• Con la revisión activada, 'Verificar y Reparar Wrapper' restaura el script de inicio modificado tras una actualización oficial de Valheim.",
                    ],
                ),
                (
                    "Opción 1: Subir a Thunderstore (Público o No Listado):",
                    &[
                        "1. Crea una cuenta en https://thunderstore.io/c/valheim/",
                        "2. Haz clic en 'Upload' en la esquina superior derecha.",
                        "3. Selecciona tu equipo, adjunta el archivo .zip generado en tu carpeta de salida y añade la etiqueta 'Modpacks'.",
                        "4. ¡Cuando los jugadores lo instalen mediante r2modman o Gale, todos los mods y configuraciones se descargarán automáticamente!",
                    ],
                ),
                (
                    "Opción 2: Compartir Directamente con Amigos:",
                    &[
                        "• Envía el archivo .zip directamente a tus amigos o extráelo en su carpeta BepInEx de Valheim.",
                    ],
                ),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_defaults_and_toggles() {
        assert_eq!(Language::default(), Language::English);
        assert_eq!(Language::English.code(), "EN");
        assert_eq!(Language::Spanish.code(), "ES");
    }

    #[test]
    fn test_translations_non_empty() {
        for lang in [Language::English, Language::Spanish] {
            assert!(!lang.app_title().is_empty());
            assert!(!lang.btn_open_output().is_empty());
            assert!(!lang.btn_build_quick().is_empty());
            assert!(!lang.tab_profile().is_empty());
            assert!(!lang.tab_server_sync().is_empty());
            assert!(!lang.heading_preview().is_empty());
            assert!(!lang.heading_logs().is_empty());
            assert!(!lang.msg_no_changes().is_empty());
            assert!(!lang.help_title().is_empty());
            assert!(!lang.help_content().is_empty());
        }
    }

    #[test]
    fn test_language_serde() {
        let json_en = serde_json::to_string(&Language::English).unwrap();
        let parsed_en: Language = serde_json::from_str(&json_en).unwrap();
        assert_eq!(parsed_en, Language::English);

        let json_es = serde_json::to_string(&Language::Spanish).unwrap();
        let parsed_es: Language = serde_json::from_str(&json_es).unwrap();
        assert_eq!(parsed_es, Language::Spanish);
    }
}
