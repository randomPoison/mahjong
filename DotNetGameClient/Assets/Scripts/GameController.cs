using System;
using System.Collections.Generic;
using System.Threading.Tasks;
using UniRx.Async;
using UnityEngine;
using UnityEngine.AddressableAssets;

public class GameController : MonoBehaviour
{
    [SerializeField]
    private AssetReference _playerPrefab = null;

    [SerializeField]
    private AssetReference _playerMovementPreviewPrefab = null;

    [SerializeField]
    private Camera _camera = null;

    private WebSocket _socket;

    private void Awake()
    {
        Debug.Assert(_playerPrefab != null, "Player prefab wasn't setup", this);
        Debug.Assert(_playerMovementPreviewPrefab != null, "Player movement preview prefab wasn't setup", this);
        Debug.Assert(_camera != null, "Camera hasn't been setup on game controller", this);
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
        _socket = await WebSocket.ConnectAsync(new Uri("ws://localhost:8088/ws/"));

        // Wait for the initial game state to come in from the server.
        //
        // TODO: Handle an exception being thrown while waiting (i.e. if we disconnect).
        // TODO: Handle serialization errors.
        var state = await _socket.RecvMessageAsync<GameStateData>();

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
                var screenPos = Input.mousePosition;
                var worldPos = _camera.ScreenToWorldPoint(new Vector3(screenPos.x, screenPos.y, _camera.transform.position.y));

                Debug.DrawLine(_camera.transform.position, worldPos);
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
            var update = await _socket.RecvMessageAsync<Message>();
        }
    }

    private void OnDestroy()
    {
        _socket.Close();
        _socket = null;
    }
}
