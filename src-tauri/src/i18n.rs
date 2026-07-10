use std::collections::HashMap;

pub struct TrayI18n {
    strings: HashMap<&'static str, &'static str>,
}

impl TrayI18n {
    pub fn new(lang: &str) -> Self {
        let mut strings = HashMap::new();
        strings.insert("open_shelfy", "Open Shelfy");
        strings.insert("open_folder", "Open watched folder");
        strings.insert("automation", "Orden automation");
        strings.insert("monitoring_status", "Monitoring {active}/{total} folders");
        match lang {
            "pl" => {
                strings.insert("quit", "Zamknij");
                strings.insert("settings", "Ustawienia");
                strings.insert("clean_now", "Posprzątaj teraz");
                strings.insert("tooltip", "Shelfy");
                strings.insert("tooltip_one_pending", "Shelfy – {} plik czeka");
                strings.insert("tooltip_many_pending", "Shelfy – {} pliki czekają");
                strings.insert("popup_title", "Shelfy");
                strings.insert("settings_title", "Ustawienia Shelfy");
                strings.insert("organized", "Uporządkowano {} plik(i)");
            }
            "it" => {
                strings.insert("quit", "Esci");
                strings.insert("settings", "Impostazioni");
                strings.insert("clean_now", "Pulisci ora");
                strings.insert("tooltip", "Shelfy");
                strings.insert("tooltip_one_pending", "Shelfy – {} file in attesa");
                strings.insert("tooltip_many_pending", "Shelfy – {} file in attesa");
                strings.insert("popup_title", "Shelfy");
                strings.insert("settings_title", "Impostazioni Shelfy");
                strings.insert("organized", "Organizzati {} file");
            }
            "de" => {
                strings.insert("quit", "Beenden");
                strings.insert("settings", "Einstellungen");
                strings.insert("clean_now", "Jetzt aufräumen");
                strings.insert("tooltip", "Shelfy");
                strings.insert("tooltip_one_pending", "Shelfy – {} Datei wartend");
                strings.insert("tooltip_many_pending", "Shelfy – {} Dateien wartend");
                strings.insert("popup_title", "Shelfy");
                strings.insert("settings_title", "Shelfy Einstellungen");
                strings.insert("organized", "{} Datei(en) organisiert");
            }
            "fr" => {
                strings.insert("quit", "Quitter");
                strings.insert("settings", "Paramètres");
                strings.insert("clean_now", "Nettoyer maintenant");
                strings.insert("tooltip", "Shelfy");
                strings.insert("tooltip_one_pending", "Shelfy – {} fichier en attente");
                strings.insert("tooltip_many_pending", "Shelfy – {} fichiers en attente");
                strings.insert("popup_title", "Shelfy");
                strings.insert("settings_title", "Paramètres Shelfy");
                strings.insert("organized", "{} fichier(s) organisé(s)");
            }
            "ru" => {
                strings.insert("quit", "Выход");
                strings.insert("settings", "Настройки");
                strings.insert("clean_now", "Очистить сейчас");
                strings.insert("tooltip", "Shelfy");
                strings.insert("tooltip_one_pending", "Shelfy – {} файл ожидает");
                strings.insert("tooltip_many_pending", "Shelfy – {} файла ожидают");
                strings.insert("popup_title", "Shelfy");
                strings.insert("settings_title", "Настройки Shelfy");
                strings.insert("organized", "Организовано {} файл(ов)");
            }
            "ja" => {
                strings.insert("quit", "終了");
                strings.insert("settings", "設定");
                strings.insert("clean_now", "今すぐ整理");
                strings.insert("tooltip", "Shelfy");
                strings.insert("tooltip_one_pending", "Shelfy – {} 個のファイルが待機中");
                strings.insert("tooltip_many_pending", "Shelfy – {} 個のファイルが待機中");
                strings.insert("popup_title", "Shelfy");
                strings.insert("settings_title", "Shelfyの設定");
                strings.insert("organized", "{}個のファイルを整理しました");
            }
            "zh" => {
                strings.insert("quit", "退出 Shelfy");
                strings.insert("settings", "设置");
                strings.insert("clean_now", "立即整理");
                strings.insert("open_shelfy", "打开 Shelfy");
                strings.insert("open_folder", "打开监控文件夹");
                strings.insert("automation", "Orden 自动化");
                strings.insert("monitoring_status", "正在监控 {active}/{total} 个文件夹");
                strings.insert("tooltip", "Shelfy 文件整理");
                strings.insert("tooltip_one_pending", "Shelfy – {} 个文件待处理");
                strings.insert("tooltip_many_pending", "Shelfy – {} 个文件待处理");
                strings.insert("popup_title", "Shelfy");
                strings.insert("settings_title", "Shelfy 设置");
                strings.insert("organized", "已整理 {} 个文件");
            }
            _ => {
                strings.insert("quit", "Quit");
                strings.insert("settings", "Settings");
                strings.insert("clean_now", "Organize Now");
                strings.insert("tooltip", "Shelfy");
                strings.insert("tooltip_one_pending", "Shelfy – {} file waiting");
                strings.insert("tooltip_many_pending", "Shelfy – {} files waiting");
                strings.insert("popup_title", "Shelfy");
                strings.insert("settings_title", "Shelfy Settings");
                strings.insert("organized", "Organized {} file(s)");
            }
        }
        Self { strings }
    }

    pub fn get<'a>(&self, key: &'a str) -> &'a str {
        self.strings.get(key).copied().unwrap_or(key)
    }
}

#[cfg(test)]
mod tests {
    use super::TrayI18n;

    #[test]
    fn chinese_tray_strings_cover_management_actions() {
        let i18n = TrayI18n::new("zh");
        assert_eq!(i18n.get("open_shelfy"), "打开 Shelfy");
        assert_eq!(i18n.get("automation"), "Orden 自动化");
        assert!(i18n.get("monitoring_status").contains("{active}"));
    }

    #[test]
    fn untranslated_tray_actions_fall_back_to_english() {
        let i18n = TrayI18n::new("de");
        assert_eq!(i18n.get("open_folder"), "Open watched folder");
    }
}
