use substrate_build_script_utils::{generate_cargo_keys, rerun_if_git_head_changed};
// bump 2
fn main() {
	generate_cargo_keys();

	rerun_if_git_head_changed();
}
