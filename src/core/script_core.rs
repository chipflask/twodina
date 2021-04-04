use artichoke;

pub mod mruby;

#[derive(Debug, Default, Clone)]
pub struct ScriptCore {}

impl artichoke::backend::convert::HeapAllocatedData for ScriptCore {
    const RUBY_TYPE: &'static str = "ScriptCore";
}
