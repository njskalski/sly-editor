use cursive;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ViewHandle {
    screenId : cursive::ScreenId,
    viewId : Option<String>
}
