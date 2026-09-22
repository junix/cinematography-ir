use super::*;

#[test]
fn blender_command_propagates_python_failures() {
    let command = blender_command(Path::new("blender"), Path::new("build.py"));
    let args: Vec<_> = command
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        args,
        [
            "--background",
            "--factory-startup",
            "--disable-autoexec",
            "--python-exit-code",
            "70",
            "--python",
            "build.py",
        ]
    );
}
