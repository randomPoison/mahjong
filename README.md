# Mahjong Prototype

This project is an prototype of a client-server game architecture, with the server component written in Rust and the client built on the Unity game engine. The core architecture uses a persistent websocket connection between client and server for communication.

The goal is to evaluate:

* How viable this is as a technology stack for mobile and web games made in Unity.
* How best to build out this tech stack.

## Setup

You'll need to have the following things installed in order to build and run this project:

* The latest version of Rust: https://rustup.rs/
* The Unity editor (currently 2019.3.0f5): https://unity3d.com/get-unity/download

## Running the Game

To run the server, navigate to the `mahjong-server` directory and then run the following command in your terminal:

```
cargo run
```

Once the server is running, open the `mahjong-client` directory in the Unity editor. Open the main scene (TBD which one that is) and hit the play button.
