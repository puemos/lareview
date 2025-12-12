use crate::ui::app::GenMsg;

#[derive(Debug)]
pub enum Action {
    Generate(GenerateAction),
    Async(AsyncAction),
}

#[derive(Debug)]
pub enum GenerateAction {
    Reset,
    RunRequested,
}

#[derive(Debug)]
pub enum AsyncAction {
    GenerationMessage(GenMsg),
}
