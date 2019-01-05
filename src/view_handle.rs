use uid;
use cursive;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ViewHandle {
    screen_id : cursive::ScreenId,
    view_id : usize
}

impl ViewHandle {
    pub fn new(screen_id : cursive::ScreenId, view_id : &uid::Id<usize>) -> Self {
        ViewHandle{ screen_id, view_id : view_id.get() }
    }

    pub fn view_id(&self) -> usize {
        self.view_id
    }

    pub fn screen_id(&self) -> cursive::ScreenId {
        self.screen_id
    }
}
