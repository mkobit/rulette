pub mod agent_skills;
pub mod backend;
pub mod claude;
pub mod cli;
pub mod frontend;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
