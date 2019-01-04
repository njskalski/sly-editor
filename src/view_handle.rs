use cursive;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ViewHandle {
    screenId : cursive::ScreenId,
    viewId : Option<String>
}
