# .NET Game Prototype

This project is an prototype of a client-server game architecture, with the server component written in .NET C# and the client using the Unity game engine. The core architecture uses a persistent websocket connection between client and server for communication.

The goal is to evaluate:

* How viable this is as a technology stack for mobile and web games made in Unity.
* How best to build out this tech stack.

## Setup

To run the server you'll need the .NET SDK. Follow the [installation instructions](https://dotnet.microsoft.com/learn/aspnet/hello-world-tutorial/install) to get that installed.

In the `DotNetGameServer` directory, run the following command in your terminal:

```
dotnet run
```

> NOTE: This project is also configured to be work with Visual Studio Code. If you open the `DotNetGameServer` folder in VS Code, you can can use the `Debug > Start Debugging` menu item to run the server and attach the debugger. You'll need the [C# extension](https://marketplace.visualstudio.com/items?itemName=ms-vscode.csharp) installed.

To run the client, open the `DotNetGameClient` project in Unity 2019.2, open the WebsocketTest scene, and hit play in the editor.

## Updating Mahjong

After making changes to the shared Mahjong project, you'll need to run the following command from within the `Mahjong` directory in order to build the DLL and copy it and its dependencies into the Unity project:

```
dotnet publish -c Release -o ..\DotNetGameClient\Packages\com.synapse-games.mahjong
```

If you've added any new Nuget dependencies, these will show up in the Unity project as new `.dll` files. If these files are not compatible with Unity, and we provide a Unity-specific alternative in the client, you'll need to change the import settings to disable it on the appropriate platforms.
