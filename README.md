A 2D game built with bevy.

### Usage

Run it with the following:

```shell
cargo run --release
```

When developing, use this to dynamically link with bevy for faster builds:

```shell
cargo run --features bevy/dynamic
```

### Controls

Gamepads and keyboard are supported.

- Player 1: WASD keys to move, left shift to run
- Player 2: Arrow keys to move, right shift to run
- Escape to exit

### Building

Build a release:

```shell
./build_release
```

Output will be `twodina.zip`.
