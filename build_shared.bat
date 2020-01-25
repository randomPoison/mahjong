REM @ECHO OFF

REM Temporary helper script to generate the dylib, generate the C# bindings, and
REM copy both over to the Unity test project. This helps speed up testing during
REM development. We should replace this with a more general (and cross-platform)
REM workflow for testing.

REM Build and install the cs-bindgen CLI so that we can run it now.

CD ../cs-bindgen

cargo install --path cs-bindgen-cli --debug --force

CD ../mahjong

REM TODO: Should cs-bindgen handle building the WASM module? For wasm-bindgen, that
REM part is handled by wasm-pack. We don't have an equivalent for cs-bindgen, maybe
REM that part would be handled by the Unity plugin since that's the closest
REM equivalent?
cargo build -p mahjong --target wasm32-unknown-unknown
cs-bindgen-cli -o mahjong-client/Packages/com.synapse-games.mahjong/Mahjong.cs target/wasm32-unknown-unknown/debug/mahjong.wasm

cargo build
XCOPY /y "target/debug/mahjong.dll" "mahjong-client/Packages/com.synapse-games.mahjong"
