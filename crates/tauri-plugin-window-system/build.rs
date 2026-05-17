use std::{collections::BTreeMap, env, path::PathBuf};

fn main() {
  let name = env::var("CARGO_PKG_NAME").expect("CARGO_PKG_NAME is not set");
  let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is not set"));

  // Keep generated permission metadata in OUT_DIR so cargo publish verification
  // never observes source tree mutations.
  println!("cargo:rerun-if-changed=permissions");

  let permissions = tauri_utils::acl::build::define_permissions(
    "./permissions/**/*.*",
    &name,
    &out_dir,
    |_| true,
  )
  .expect("failed to define plugin permissions");

  let mut permissions_map = BTreeMap::new();
  permissions_map.insert(name, permissions);

  tauri_utils::acl::build::generate_allowed_commands(&out_dir, None, permissions_map)
    .expect("failed to generate allowed commands");
}
