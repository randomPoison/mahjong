using System;
using System.Threading.Tasks;
using UniRx.Async;
using UnityEngine;

public class StartupController : MonoBehaviour
{
    private WebSocket _socket;

    private async void Start()
    {
        try
        {
            await DoMainLoop();
        }
        catch (TaskCanceledException exception)
        {
            Debug.LogFormat($"Main task was canceled: {exception}");
        }
    }

    private async UniTask DoMainLoop()
    {
        // TODO: Handle an exception being thrown as a result of the connection failing.
        // TODO: Make server address configurable.
        _socket = await WebSocket.ConnectAsync(new Uri("ws://localhost:3030/client"));

        Debug.Log("Established connection with server, beginning handshake");

        // Perform handshake and initialization sequence with the server:
        //
        // * The client sends account ID and initial configuration data.
        // * Server sends current account data and any updated cache data.
        var requestString = Mahjong.CreateHandshakeRequest();
        _socket.SendString(requestString);

        // TODO: Send cached account ID to server, or request new account.

        // TODO: Receive account data from server.

        // Once the initial state has been received from the server, spawn two tasks to
        // run concurrently:
        //
        // * One to listen for and handle incoming messages from the server.
        // * One to handle player input every frame.
        var handleMessages = HandleMessages();
        var handleInput = HandleInput();

        await UniTask.WhenAll(handleMessages, handleInput);
    }

    private async UniTask HandleInput()
    {
        while (true)
        {
            if (Input.GetMouseButton(0))
            {
            }

            await UniTask.Yield();
        }
    }

    private async UniTask HandleMessages()
    {
        while (true)
        {
            // TODO: Handle an exception being thrown while waiting (i.e. if we disconnect).
            // TODO: Handle serialization errors.
            var message = await _socket.RecvStringAsync();
            Debug.Log($"Received message: {message}");
        }
    }

    private void OnDestroy()
    {
        _socket?.Close();
        _socket = null;
    }
}
