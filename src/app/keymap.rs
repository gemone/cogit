use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::panels::{Action, Mode};

pub struct KeyMap {
    pub global: HashMap<(KeyModifiers, KeyCode), Action>,
    pub filelist: HashMap<(KeyModifiers, KeyCode), Action>,
    pub sidebar: HashMap<(KeyModifiers, KeyCode), Action>,
    pub diff: HashMap<(KeyModifiers, KeyCode), Action>,
    pub command: HashMap<(KeyModifiers, KeyCode), Action>,
}

impl Default for KeyMap {
    fn default() -> Self {
        let mut global = HashMap::new();
        global.insert((KeyModifiers::NONE, KeyCode::Tab), Action::FocusFilelist);
        global.insert((KeyModifiers::SHIFT, KeyCode::BackTab), Action::FocusSidebar);
        global.insert((KeyModifiers::NONE, KeyCode::Char('1')), Action::FocusSidebar);
        global.insert((KeyModifiers::NONE, KeyCode::Char('2')), Action::FocusFilelist);
        global.insert((KeyModifiers::NONE, KeyCode::Char('3')), Action::FocusDiff);
        global.insert((KeyModifiers::NONE, KeyCode::Char(':')), Action::CommandPalette);
        global.insert((KeyModifiers::NONE, KeyCode::Char('?')), Action::Help);
        global.insert((KeyModifiers::NONE, KeyCode::Char('q')), Action::Quit);

        let mut filelist = HashMap::new();
        filelist.insert((KeyModifiers::NONE, KeyCode::Char(' ')), Action::Stage);
        filelist.insert((KeyModifiers::NONE, KeyCode::Char('s')), Action::Stage);
        filelist.insert((KeyModifiers::NONE, KeyCode::Char('u')), Action::Unstage);
        filelist.insert((KeyModifiers::NONE, KeyCode::Char('a')), Action::StageAll);
        filelist.insert((KeyModifiers::SHIFT, KeyCode::Char('A')), Action::UnstageAll);
        filelist.insert((KeyModifiers::NONE, KeyCode::Char('d')), Action::Discard);
        filelist.insert((KeyModifiers::NONE, KeyCode::Enter), Action::FocusDiff);
        filelist.insert((KeyModifiers::NONE, KeyCode::Char('i')), Action::ToggleIgnore);
        filelist.insert((KeyModifiers::SHIFT, KeyCode::Char('I')), Action::ToggleIgnore);
        filelist.insert((KeyModifiers::SHIFT, KeyCode::Char('U')), Action::ToggleUntracked);
        filelist.insert((KeyModifiers::NONE, KeyCode::Char('/')), Action::CommandPalette);

        let mut sidebar = HashMap::new();
        sidebar.insert((KeyModifiers::NONE, KeyCode::Char('c')), Action::Checkout);
        sidebar.insert((KeyModifiers::NONE, KeyCode::Char('b')), Action::NewBranch);
        sidebar.insert((KeyModifiers::SHIFT, KeyCode::Char('D')), Action::DeleteBranch);
        sidebar.insert((KeyModifiers::NONE, KeyCode::Char('o')), Action::None);
        sidebar.insert((KeyModifiers::NONE, KeyCode::Char('r')), Action::Refresh);

        let mut diff = HashMap::new();
        diff.insert((KeyModifiers::NONE, KeyCode::Char('s')), Action::Stage);
        diff.insert((KeyModifiers::NONE, KeyCode::Char('u')), Action::Unstage);

        let mut command = HashMap::new();
        command.insert((KeyModifiers::NONE, KeyCode::Enter), Action::None);
        command.insert((KeyModifiers::NONE, KeyCode::Esc), Action::EnterMode(Mode::Normal));

        Self {
            global,
            filelist,
            sidebar,
            diff,
            command,
        }
    }
}

impl KeyMap {
    pub fn resolve(&self, mode: Mode, panel: &str, key: KeyEvent) -> Action {
        if mode == Mode::Command {
            return self
                .command
                .get(&(key.modifiers, key.code))
                .cloned()
                .unwrap_or(Action::None);
        }

        let map = match panel {
            "sidebar" => &self.sidebar,
            "filelist" => &self.filelist,
            "diff" => &self.diff,
            _ => &self.global,
        };

        if let Some(action) = map.get(&(key.modifiers, key.code)) {
            return action.clone();
        }

        if let Some(action) = self.global.get(&(key.modifiers, key.code)) {
            return action.clone();
        }

        Action::None
    }
}
