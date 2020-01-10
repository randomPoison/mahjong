using System;
using System.Collections.Generic;
using System.Threading.Tasks;
using DotNetGame.Mahjong;
using JsonSubTypes;
using Newtonsoft.Json;
using UniRx.Async;
using UnityEngine;
using UnityEngine.Assertions;

[JsonConverter(typeof(JsonSubtypes), "Type")]
[JsonSubtypes.KnownSubType(typeof(TestClassA), "A")]
[JsonSubtypes.KnownSubType(typeof(TestClassB), "B")]
public interface ITestInterface
{
    string Type { get; }
}

public class TestClassA : ITestInterface
{
    public string Type => "A";
}

public class TestClassB : ITestInterface
{
    public string Type => "B";
}

public class WebSocketTestController : MonoBehaviour
{
    private WebSocket _socket;

    private void Awake()
    {
        {
            ITestInterface test = new TestClassB();
            var json = JsonConvert.SerializeObject(test);

            var result = JsonConvert.DeserializeObject<ITestInterface>(json);
            if (result is TestClassB b)
            {
                Debug.Log("Deserialized successfully");
            }
            else
            {
                Debug.LogError("Deserialization didn't work :'(");
            }
        }

        {
            ITile tile = new SimpleTile(Suit.Coins, 1);
            var json = JsonConvert.SerializeObject(tile);

            ITile result = JsonConvert.DeserializeObject<ITile>(json);
            SimpleTile resultTile = (SimpleTile)result;
            Assert.AreEqual(Suit.Coins, resultTile.Suit);
            Assert.AreEqual(1, resultTile.Number);
        }
    }

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
        // Wait to receive the initial set of tiles. The world sends this first thing
        // after establishing a connection.
        {
            var message = await _socket.RecvStringAsync();
            Debug.Log($"Received message: {message}", this);

            var tiles = JsonConvert.DeserializeObject<List<ITile>>(message);
            Debug.Log($"Deserialized initial tile set: {tiles}", this);
        }

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
