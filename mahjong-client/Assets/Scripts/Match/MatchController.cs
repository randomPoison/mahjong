using Synapse.Utils;
using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading;
using UniRx.Async;
using UnityEngine;
using UnityEngine.AddressableAssets;
using UnityEngine.UI;

namespace Synapse.Mahjong.Match
{
    /// <summary>
    /// Main controller for the mahjong gameplay.
    /// </summary>
    public class MatchController : MonoBehaviour
    {
        #region Configuration Fields

        [SerializeField]
        [Tooltip(
            "The root object for each player's hand. Tiles in each players' hand will " +
            "be made children of these objects.")]
        private Transform[] _handRoots = default;

        [SerializeField] private LocalHandView _localHandPrefab = default;
        [SerializeField] private RemoteHandView _remoteHandPrefab = default;

        // TODO: Move the tile asset configuration into a scriptable object. While we
        // only have one set of tile assets we can get away with baking it directly into
        // the controller, but this setup won't scale.
        [Header("Tile Assets")]
        [SerializeField] private AssetReferenceGameObject[] _bambooTiles = default;
        [SerializeField] private AssetReferenceGameObject[] _circleTiles = default;
        [SerializeField] private AssetReferenceGameObject[] _characterTiles = default;
        [SerializeField] private AssetReferenceGameObject[] _dragonTiles = default;
        [SerializeField] private AssetReferenceGameObject[] _windTiles = default;
        [SerializeField] private AssetReferenceGameObject _dummyTile = default;

        [Header("UI Elements")]
        [SerializeField] private GameObject _matchEndedDisplayRoot = default;
        [SerializeField] private Button _exitButton = default;
        [SerializeField] private CallDiscardPrompt _callDiscardPrompt = default;

        [Header("Action Timing")]
        [SerializeField] private float _delayAfterDraw = 1f;
        [SerializeField] private float _delayAfterDiscard = 1f;

        #endregion

        #region Private Fields

        private WebSocket _socket;
        private ClientState _client;

        // TODO: Move the tracking for sever state vs local state into the Rust layer.
        // It'll be easier to write and test the reconciliation logic there, and would
        // allow us to simplify the client code.
        private LocalState _serverState;
        private LocalState _localState;

        private Wind _seat;

        private PlayerHandView[] _hands = new PlayerHandView[4];

        // Tracking for the most recent discard action that the player performed, used
        // to handle forward-simulation and state verification after the client receives
        // match updates from the server.
        private TileId? _lastDiscard;

        // Cached prefabs for the different tiles. These are populated during startup
        // based on the asset references configured above.
        private GameObject[] _bambooPrefabs = new GameObject[9];
        private GameObject[] _circlePrefabs = new GameObject[9];
        private GameObject[] _characterPrefabs = new GameObject[9];
        private GameObject[] _dragonPrefabs = new GameObject[3];
        private GameObject[] _windPrefabs = new GameObject[4];
        private GameObject _dummyPrefab = null;

        // Root cancellation source for any tasks spawned by the controller, used to
        // cancel all pending tasks if we abruptly exit the match screen.
        private CancellationTokenSource _cancellation = new CancellationTokenSource();

        #endregion

        public async UniTask<NextScreen> Run(ClientState client, WebSocket socket)
        {
            _client = client;
            _socket = socket;

            // Request that the server start a new match, and load our tile prefabs in
            // the background. Wait for both of these operations to complete before
            // attempting to instantiate any tiles.
            await UniTask.WhenAll(
                RequestStartMatch(_cancellation.Token),
                LoadTilePrefabs(_cancellation.Token));

            // TODO: Have the server data specify which player is controlled by this
            // client, rather than hard coding it to always control the east seat.
            _seat = Wind.East;

            // Register input events from the player's hand.
            var playerHand = _hands[(int)_seat];

            // Once we have the match data, instantiate the tiles for each player's
            // starting hand.
            foreach (var seat in EnumUtils.GetValues<Wind>())
            {
                if (seat == _localState.Seat())
                {
                    // Instantiate the local hand view controller.
                    var view = Instantiate(_localHandPrefab, _handRoots[(int)seat]);
                    _hands[(int)seat] = view;

                    // Populate the hand with its initial state.

                    var handState = _localState.LocalHand(seat);
                    var tiles = handState.GetTiles();

                    foreach (var tile in tiles)
                    {
                        view.AddToHand(InstantiateTile(tile));
                    }

                    if (handState.HasCurrentDraw())
                    {
                        var currentDraw = handState.GetCurrentDraw();
                        await view.DrawTile(InstantiateTile(currentDraw));
                    }
                }
                else
                {
                    // Instantiate the remote hand view controller.
                    var view = Instantiate(_remoteHandPrefab, _handRoots[(int)seat]);
                    _hands[(int)seat] = view;

                    // Populate the hand with its initial state.

                    view.FillWithDummyTiles(_dummyPrefab);

                    if (_localState.PlayerHasCurrentDraw(seat))
                    {
                        view.DrawDummyTile(_dummyPrefab);
                    }
                }

            }

            // Run the core loop of the match, alternating between performing any
            // actions necessary for the current turn state and processing the next
            // update coming from the server.
            var matchEnded = false;
            while (!matchEnded)
            {
                // Check the current state of the match and determine if there's any action that
                // the local player needs to take.
                // --------------------------------------------------------------------------------
                var turnState = _localState.TurnState();
                Debug.Log($"Handling current turn state: {turnState}");

                switch (turnState)
                {
                    // If it's the current player's turn to discard, wait for the player
                    // to choose their discard tile.
                    case LocalTurnState.AwaitingDiscard awaitingDiscard:
                    {
                        if (awaitingDiscard.Element0 == _seat)
                        {
                            await DiscardTile();
                        }
                    }
                    break;

                    // If the player can call the last discarded tile, give them a
                    // chance to choose if they want to call the tile or pass.
                    case LocalTurnState.AwaitingCalls awaitingCalls:
                    {
                        if (awaitingCalls.Calls.Count > 0)
                        {
                            _callDiscardPrompt.gameObject.SetActive(true);
                            var selectedCall = await _callDiscardPrompt.MakeCall(awaitingCalls.Calls, _cancellation.Token);
                            _callDiscardPrompt.gameObject.SetActive(false);

                            // Send the selected call action to the server.
                            string request;
                            if (selectedCall != null)
                            {
                                request = _localState.RequestCallTile(selectedCall);
                            }
                            else
                            {
                                request = _localState.RequestPass();
                            }
                            _socket.SendString(request);
                        }
                    }
                    break;

                    case LocalTurnState.AwaitingDraw _:
                    case LocalTurnState.MatchEnded _:
                        break;

                    default:
                        throw new NotImplementedException($"Unhandled turn state: {turnState}");
                }

                // Wait to receive the next update from the server and merge it in with the server
                // state once received.
                // --------------------------------------------------------------------------------
                var eventJson = await _socket.RecvStringAsync(_cancellation.Token);
                IMatchEvent update = _serverState.DeserializeAndHandleEvent(eventJson);
                Debug.Log($"Handling incoming update: {update}");

                // Apply the received update to the local state, updating both the game
                // state tracking and the visual state.
                switch (update)
                {
                    case MatchEvent.LocalDraw localDraw:
                    {
                        Debug.Assert(
                            localDraw.Seat == _seat,
                            $"Seat for `LocalDraw` event was {localDraw.Seat}, but local seat is {_seat}",
                            this);

                        // Update the game state tracking for the client.
                        if (!_localState.TryDrawLocalTile(localDraw.Tile))
                        {
                            throw new OutOfSyncException(
                                $"Unable to perform local draw of tile {localDraw.Tile}");
                        }

                        var currentDraw = _localState.LocalHand(_seat).GetCurrentDraw();

                        // Update the visuals based on the draw event.
                        var view = (LocalHandView)_hands[(int)_seat];
                        var tileObject = InstantiateTile(currentDraw);
                        await view.DrawTile(tileObject);
                    }
                    break;

                    case MatchEvent.RemoteDraw remoteDraw:
                    {
                        if (!_localState.TryDrawRemoteTile(remoteDraw.Seat))
                        {
                            throw new OutOfSyncException(
                                $"Unable to perform draw for remote player {remoteDraw.Seat}");
                        }

                        // Update the visuals based on the draw event.
                        var view = (RemoteHandView)_hands[(int)remoteDraw.Seat];
                        view.DrawDummyTile(_dummyPrefab);
                    }
                    break;

                    case MatchEvent.TileDiscarded discard:
                    {
                        // If we performed a discard event locally, the next discard event from
                        // the server should match the one we performed. Verify that's the case
                        // and reconcile our local state with the server state.
                        //
                        // Otherwise, perform the action on the local state and then verify that
                        // the local state is still in sync with the server state.
                        if (discard.Seat == _seat)
                        {
                            if (_lastDiscard == null
                                || discard.Tile.Element0 != _lastDiscard.Value.Element0)
                            {
                                throw new OutOfSyncException(
                                    $"Previously discarded tile {_lastDiscard}, but received" +
                                    $"discard event {discard}");
                            }

                            // Clear local tracking for discarded tile now that the server has
                            // caught up.
                            _lastDiscard = null;
                        }
                        else if (_localState.TryDiscardTile(discard.Seat, discard.Tile))
                        {
                            // Perform the discard action locally.
                            switch (_hands[(int)discard.Seat])
                            {
                                case LocalHandView localHand:
                                {
                                    localHand.MoveToDiscard(discard.Tile);
                                }
                                break;

                                case RemoteHandView remoteHand:
                                {
                                    remoteHand.DiscardTile(InstantiateTile(global::Mahjong.InstanceById(discard.Tile)));
                                }
                                break;
                            }

                            // TODO: Remove the explicit delay once we have a proper animation.
                            await UniTask.Delay((int)(_delayAfterDiscard * 1000));
                        }
                        else
                        {
                            throw new OutOfSyncException($"Could not apply discard event locally: {discard}");
                        }

                        // TODO: Reconcile our local state with the updated server state to
                        // verify that the two are in sync.
                    }
                    break;

                    case MatchEvent.Pass _:
                    {
                        if (!_localState.TryDecidePass())
                        {
                            throw new OutOfSyncException($"Could not apply pass event to local state");
                        }
                    }
                    break;

                    // A player made a call. If it was the local player, we reconcile the event
                    // with our local state, ensuring that the update from the server matches the
                    // call that we made locally. If it was a remote player, we update our local
                    // state.
                    case MatchEvent.Call callEvent:
                    {
                        var call = callEvent.Element0;

                        if (!_localState.TryDecideCall(call))
                        {
                            throw new OutOfSyncException($"Unable to locally decide call: {call}");
                        }

                        // TODO: Visualize the call, i.e. remove the tile from the discarding
                        // player's hand and form the open meld in the calling player's hand.
                        var discardingHand = _hands[(int)call.CalledFrom];
                        var discardView = discardingHand.RemoveLastDiscard();

                        if (discardView.Model.Id.Element0 != call.Discard.Element0)
                        {
                            throw new OutOfSyncException(
                                $"Attempting to call discarded tile {call.Discard}, but last " +
                                $"discarded tile for {call.CalledFrom} was {discardView.Model.Id}");
                        }

                        switch (_hands[(int)call.Caller])
                        {
                            case LocalHandView localHand:
                            {
                                localHand.CallTile(discardView, call.WinningCall);
                            }
                            break;

                            case RemoteHandView remoteHand:
                            {
                                remoteHand.CallTile(
                                    discardView,
                                    call.WinningCall,
                                    id => InstantiateTile(global::Mahjong.InstanceById(id)));
                            }
                            break;
                        }
                    }
                    break;

                    case MatchEvent.MatchEnded _:
                    {
                        matchEnded = true;
                    }
                    break;

                    default:
                        throw new NotImplementedException($"Unhandled match event: {update}");
                }
            }

            // Display that the match ended UI and wait for the player to hit the exit button.
            _matchEndedDisplayRoot.SetActive(true);
            await _exitButton.OnClickAsync(_cancellation.Token);

            // Exit the match, indicating that we should return to the home screen.
            return NextScreen.Home;

            // Helper method to handle requesting match creation from the server.
            async UniTask RequestStartMatch(CancellationToken cancellation = default)
            {
                // Request that the server start a match.
                var request = _client.CreateStartMatchRequest();
                _socket.SendString(request);
                var responseJson = await _socket.RecvStringAsync(cancellation);

                // TODO: Add some kind of error handling around failure. Probably not doable
                // until we can return more structured data from Rust functions.
                _serverState = _client.HandleStartMatchResponse(responseJson);

                // TODO: Clone the server state directly to get the initial local state.
                // This will require cs-bindgen to generate `Clone()` methods. For now
                // we'll have to re-deserialize the server response to get a fresh copy
                // of the match state.
                _localState = _client.HandleStartMatchResponse(responseJson);

                Debug.Log($"Started match, ID: {_serverState.Id()}", this);
            }
        }

        /// <summary>
        /// Waits for the player to select a tile to discard, then performs the discard.
        /// </summary>
        ///
        /// <returns>
        /// A task that resolves once the discard action has finished.
        /// </returns>
        ///
        /// <remarks>
        /// 
        /// </remarks>
        private async UniTask DiscardTile()
        {
            var hand = (LocalHandView)_hands[(int)_seat];
            var id = await hand.OnClickTileAsync(_cancellation.Token);

            // Attempt to discard the tile. If the operation fails, ignore the click event.
            if (!_localState.TryDiscardTile(_seat, id))
            {
                throw new NotImplementedException(
                    "What does it mean if the tile click fails here?");
            }

            // Track which tile we've discarded locally. This will allow us to reconcile
            // our local state with the server's state once we receive updates from the
            // server.
            Debug.Assert(
                !_lastDiscard.HasValue,
                "Discarding a tile when the last discard still hasn't been processed",
                this);
            _lastDiscard = id;

            // Send the tile to the graveyard!
            hand.MoveToDiscard(id);

            // If the local attempt to discard the tile succeeded, send a request to the
            // server to perform the action.
            var request = _serverState.RequestDiscardTile(id);
            _socket.SendString(request);
        }

        #region Unity Lifecycle Methods

        private void OnDestroy()
        {
            _serverState?.Dispose();
            _serverState = null;

            _localState?.Dispose();
            _localState = null;

            // Cancel any pending tasks.
            _cancellation.Cancel();
            _cancellation.Dispose();
            _cancellation = null;

            // TODO: Unload any tile assets that we loaded? This *might* require some
            // logic to only attempt to unload assets that were successfully loaded in
            // the case where not all tile assets loaded before we leave the match.
        }

        #endregion

        #region Tile Asset Handling

        /// <summary>
        /// Loads the prefabs for all of the tiles and populates the prefab lists.
        /// </summary>
        ///
        /// <returns>
        /// A task that resolves once all prefabs have finished loading.
        /// </returns>
        private async UniTask LoadTilePrefabs(CancellationToken cancellation = default)
        {
            var tasks = new List<UniTask>();

            foreach (var (index, asset) in _bambooTiles.Enumerate())
            {
                tasks.Add(LoadSingleAsset(
                    asset,
                    prefab =>
                    {
                        _bambooPrefabs[index] = prefab;
                    },
                    cancellation));
            }

            foreach (var (index, asset) in _characterTiles.Enumerate())
            {
                tasks.Add(LoadSingleAsset(
                    asset,
                    prefab =>
                    {
                        _characterPrefabs[index] = prefab;
                    },
                    cancellation));
            }

            foreach (var (index, asset) in _circleTiles.Enumerate())
            {
                tasks.Add(LoadSingleAsset(
                    asset,
                    prefab =>
                    {
                        _circlePrefabs[index] = prefab;
                    },
                    cancellation));
            }

            foreach (var dragon in EnumUtils.GetValues<Dragon>())
            {
                var asset = _dragonTiles[(int)dragon];
                tasks.Add(LoadSingleAsset(
                    asset,
                    prefab =>
                    {
                        _dragonPrefabs[(int)dragon] = prefab;
                    },
                    cancellation));
            }


            foreach (var wind in EnumUtils.GetValues<Wind>())
            {
                var asset = _windTiles[(int)wind];
                tasks.Add(LoadSingleAsset(
                    asset,
                    prefab =>
                    {
                        _windPrefabs[(int)wind] = prefab;
                    },
                    cancellation));
            }

            tasks.Add(LoadSingleAsset(
                _dummyTile,
                prefab => { _dummyPrefab = prefab; },
                cancellation));

            // Wait for all of the load operations to complete.
            await UniTask.WhenAll(tasks.ToArray());

            // Verify that all prefabs have been loaded and correctly cached.
            Debug.Assert(
                _bambooPrefabs.All(prefab => prefab != null),
                "Not all bamboo tile prefabs loaded");
            Debug.Assert(
                _characterPrefabs.All(prefab => prefab != null),
                "Not all character tile prefabs loaded");
            Debug.Assert(
                _circlePrefabs.All(prefab => prefab != null),
                "Not all circle tile prefabs loaded");
            Debug.Assert(
                _dragonPrefabs.All(prefab => prefab != null),
                "Not all dragon tile prefabs loaded");
            Debug.Assert(
                _windPrefabs.All(prefab => prefab != null),
                "Not all wind tile prefabs loaded");
            Debug.Assert(
                _dummyPrefab != null,
                "Dummy tile prefab not loaded");

            // Helper method for loading a single asset and performing some processing
            // operation (in this case adding it to the appropriate prefab list) once
            // the load completes.
            async UniTask LoadSingleAsset(
                AssetReferenceGameObject asset,
                Action<GameObject> processAsset,
                CancellationToken token = default)
            {
                var prefab = await asset
                    .LoadAssetAsync()
                    .Task
                    .WithCancellation(token);
                processAsset(prefab);
            }
        }

        private TileView InstantiateTile(TileInstance instance)
        {
            var prefab = GetPrefab(instance.Tile);
            var view = Instantiate(prefab).GetComponent<TileView>();
            view.Populate(instance);
            return view;

            // Helper method to ensure that the switch statement handles all possible
            // control flow paths. If we don't wrap it in a function, the compiler
            // wouldn't warn us if we failed to handle all control flow paths. This
            // could be replaced with a switch expression if we ever get C# 8.0 support.
            GameObject GetPrefab(ITile tile)
            {
                switch (tile)
                {
                    case Tile.Simple simple:
                        // Tile numbers start at 1, so we need to subtract 1 to get the
                        // index corresponding to the tile's numeric value.
                        int tileIndex = simple.Element0.Number - 1;

                        switch (simple.Element0.Suit)
                        {
                            case Suit.Bamboo: return _bambooPrefabs[tileIndex];
                            case Suit.Characters: return _characterPrefabs[tileIndex];
                            case Suit.Coins: return _circlePrefabs[tileIndex];

                            default:
                                throw new ArgumentException(
                           $"Invalid suit {simple.Element0.Suit}");
                        }

                    case Tile.Dragon dragon:
                        return _dragonPrefabs[(int)dragon.Element0];

                    case Tile.Wind wind:
                        return _windPrefabs[(int)wind.Element0];

                    default: throw new ArgumentException($"Invalid tile kind {tile}");
                }
            }
        }

        #endregion
    }
}
