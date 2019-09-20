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
            Debug.LogFormat("Main task was cancelled: {0}", exception);
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
        Debug.LogFormat("Recieved initial state: {0}", state);
        Debug.LogFormat("Received initial state with {0} players", state.Players.Count);

        var players = new Dictionary<int, GameObject>();
        var movementPreviews = new Dictionary<int, GameObject>();

        // Create objects in the world as necessary based on the initial game state
        // when we first connect to the server.
        foreach (var (id, player) in state.Players)
        {
            // Create an object in the world for the player and set it to the world position
            // that corresponds to their grid position.
            var playerInstance = await Addressables.Instantiate<GameObject>(_playerPrefab);
            playerInstance.transform.localPosition = player.Pos.WorldPos;

            players.Add(id, playerInstance);

            // Visualize the pending move action for the player, if they already have
            // one setup.
            var pendingMovement = player.PendingTurn?.Movement;
            if (pendingMovement.HasValue)
            {
                var movementPreview = await Addressables.Instantiate<GameObject>(_playerMovementPreviewPrefab);
                movementPreview.transform.localPosition = pendingMovement.Value.WorldPos;

                movementPreviews.Add(id, movementPreview);
            }
        }

        // Once the intial state has been received from the server, spawn two tasks to
        // run concurrently:
        //
        // * One to listen for and handle incoming messages from the server.
        // * One to handle player input every frame.
        var handleMessages = HandleMessages(players, movementPreviews);
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

    private async UniTask HandleMessages(Dictionary<int, GameObject> players, Dictionary<int, GameObject> movementPreviews)
    {
        while (true)
        {
            // TODO: Handle an exception being thrown while waiting (i.e. if we disconnect).
            // TODO: Handle serialization errors.
            var update = await _socket.RecvMessageAsync<Message>();
            switch (update.Type)
            {
                case MessageType.PlayerAdded:
                    var playerAdded = update.Data.ToObject<PlayerAdded>();

                    // Create an object in the world for the player and set it to the world position
                    // that corresponds to their grid position.
                    var playerInstance = await Addressables.Instantiate<GameObject>(_playerPrefab);
                    playerInstance.transform.localPosition = playerAdded.Data.Pos.WorldPos;

                    Debug.AssertFormat(!players.ContainsKey(playerAdded.Id), "Player with ID {0} already exists", playerAdded.Id);
                    players.Add(playerAdded.Id, playerInstance);

                    break;

                case MessageType.SetMovement:
                    var setMovement = update.Data.ToObject<SetMovement>();

                    // Get the existing preview object, or create a new one if one doesn't
                    // already exist.
                    GameObject movementPreview;
                    if (!movementPreviews.TryGetValue(setMovement.Id, out movementPreview))
                    {
                        movementPreview = await Addressables.Instantiate<GameObject>(_playerMovementPreviewPrefab);
                        movementPreviews.Add(setMovement.Id, movementPreview);
                    }

                    movementPreview.transform.localPosition = setMovement.Pos.WorldPos;

                    break;
            }
        }
    }

    private void OnDestroy()
    {
        _socket.Close();
        _socket = null;
    }
}
