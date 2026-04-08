use crate::ir::Skill;
use anyhow::Result;

pub trait Emitter {
    fn emit(&self, skill: &Skill) -> Result<String>;
}

pub struct GeminiEmitter;
pub struct ClaudeEmitter;
pub struct CursorEmitter;

impl Emitter for GeminiEmitter {
    fn emit(&self, _skill: &Skill) -> Result<String> {
        todo!("implement gemini json emission")
    }
}

impl Emitter for ClaudeEmitter {
    fn emit(&self, _skill: &Skill) -> Result<String> {
        todo!("implement claude tools emission")
    }
}

impl Emitter for CursorEmitter {
    fn emit(&self, _skill: &Skill) -> Result<String> {
        todo!("implement cursor .mdc emission")
    }
}
