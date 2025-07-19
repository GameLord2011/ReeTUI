pub const ICONS: [&str; 11] = ["󰱨", "󰱩", "󱃞", "󰱫", "󰱬", "󰱮", "󰱰", "󰽌", "󰱱", "󰱸", "󰇹"];
#[derive(Default)]
pub struct CreateChannelForm {
    pub name: String,
    pub input_focused: CreateChannelInput,
    pub selected_icon_index: usize,
}

#[derive(PartialEq, Default, Clone, Copy)]
pub enum CreateChannelInput {
    #[default]
    Name,
    Icon,
    CreateButton,
}

impl CreateChannelForm {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            input_focused: CreateChannelInput::Name,
            selected_icon_index: 0,
        }
    }

    pub fn next_input(&mut self) {
        self.input_focused = match self.input_focused {
            CreateChannelInput::Name => CreateChannelInput::Icon,
            CreateChannelInput::Icon => CreateChannelInput::CreateButton,
            CreateChannelInput::CreateButton => CreateChannelInput::Name,
        };
    }

    pub fn previous_input(&mut self) {
        self.input_focused = match self.input_focused {
            CreateChannelInput::Name => CreateChannelInput::CreateButton,
            CreateChannelInput::Icon => CreateChannelInput::Name,
            CreateChannelInput::CreateButton => CreateChannelInput::Icon,
        };
    }

    pub fn next_icon(&mut self) {
        self.selected_icon_index = (self.selected_icon_index + 1) % ICONS.len();
    }

    pub fn previous_icon(&mut self) {
        self.selected_icon_index = (self.selected_icon_index + ICONS.len() - 1) % ICONS.len();
    }

    pub fn get_selected_icon(&self) -> String {
        ICONS[self.selected_icon_index].to_string()
    }
}

