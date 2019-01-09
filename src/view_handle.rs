use cursive;
use uid;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ViewHandle {
    view_id : usize,
}

impl ViewHandle {
    pub fn new(view_id : &uid::Id<usize>) -> Self {
        ViewHandle { view_id : view_id.get() }
    }

    pub fn view_id(&self) -> usize {
        self.view_id
    }
}
