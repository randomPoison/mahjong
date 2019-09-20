using System;
using System.Threading.Tasks;
using UniRx.Async;
using UnityEngine;

public class WebSocketTestController : MonoBehaviour
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
            Debug.LogFormat("Main task was canceled: {0}", exception);
        }
    }

    private async UniTask DoMainLoop()
    {
        // TODO: Handle an exception being thrown as a result of the connection failing.
        _socket = await WebSocket.ConnectAsync(new Uri("ws://localhost:5000/ws"));

        // Send a test message.
        _socket.SendString("Connected to server!");

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
        if (_socket != null)
        {
            _socket.Close();
            _socket = null;
        }
    }
}
