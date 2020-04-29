using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading;
using Synapse.Utils;
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
        private PlayerHand[] _hands = default;

        // TODO: Move the tile asset configuration into a scriptable object. While we
        // only have one set of tile assets we can get away with baking it directly into
        // the controller, but this setup won't scale.
        [Header("Tile Assets")]
        [SerializeField] private AssetReferenceGameObject[] _bambooTiles = default;
        [SerializeField] private AssetReferenceGameObject[] _circleTiles = default;
        [SerializeField] private AssetReferenceGameObject[] _characterTiles = default;
        [SerializeField] private AssetReferenceGameObject[] _dragonTiles = default;
        [SerializeField] private AssetReferenceGameObject[] _windTiles = default;

        [Header("UI Elements")]
        [SerializeField] private GameObject _matchEndedDisplayRoot = default;
        [SerializeField] private Button _exitButton = default;

        #endregion

        #region Private Fields

        private WebSocket _socket;
        private ClientState _client;

        // TODO: Move the tracking for sever state vs local state into the Rust layer.
        // It'll be easier to write and test the reconciliation logic there, and would
        // allow us to simplify the client code.
        private MatchState _serverState;
        private MatchState _localState;

        private Wind _seat;

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

        // Root cancellation source for any tasks spawned by the controller, used to
        // cancel all pending tasks if we abruptly exit the match screen.
        private CancellationTokenSource _cancellation = new CancellationTokenSource();

        #endregion

        public async void Init(ClientState client, WebSocket socket)
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
            //
            // TODO: Move tile placement logic into `PlayerHand`. The match controller
            // should only need to add and remove tiles from the hands as the match's
            // state advances, and the `PlayerHand` script should handle layout and
            // positioning.
            foreach (var seat in EnumUtils.GetValues<Wind>())
            {
                var hand = _hands[(int)seat];

                var tiles = _localState.PlayerHand(seat);

                foreach (var tile in tiles)
                {
                    hand.AddToHand(InstantiateTile(tile));
                }

                if (_localState.PlayerHasCurrentDraw(seat))
                {
                    var currentDraw = _localState.CurrentDraw(seat);
                    hand.DrawTile(InstantiateTile(currentDraw));
                }
            }

            // If the local player has the first turn, have them discard a tile now.
            if (_localState.CurrentTurn() == _seat)
            {
                await DiscardTile();
            }

            // Process incoming updates from the server.
            var matchEnded = false;
            while (!matchEnded)
            {
                // Wait to receive the next update from the server.
                var eventJson = await _socket.RecvStringAsync(_cancellation.Token);
                Debug.Log(eventJson, this);

                // Feed the incoming event into the server state.
                IMatchEvent update = _serverState.HandleEvent(eventJson);

                // Apply the received update to the local state, updating both the game
                // state tracking and the visual state.
                switch (update)
                {
                    case MatchEvent.TileDrawn draw:
                    {
                        // Update the game state tracking for the client.
                        if (!_localState.TryDrawTile(draw.Seat))
                        {
                            // TODO: Handle the client being out of sync with the server.
                            throw new NotImplementedException("Client out of sync with server");
                        }

                        var localDraw = _localState.CurrentDraw(draw.Seat);
                        Debug.Assert(
                            draw.Tile.Element0 == localDraw.Id.Element0,
                            "Drew incorrect tile when simulating locally",
                            this);

                        // Update the visuals based on the draw event.
                        var hand = _hands[(int)draw.Seat];
                        var tileObject = InstantiateTile(localDraw);
                        hand.DrawTile(tileObject);

                        // If the local player was the one that drew the tile, have them discard a
                        // tile now.
                        if (draw.Seat == _seat)
                        {
                            Debug.Assert(
                                _localState.CurrentTurn() == _seat,
                                "Player drew a tile but it's not their turn???");

                            await DiscardTile();
                        }
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
                        if (_lastDiscard is TileId lastDiscard)
                        {
                            if (discard.Seat != _seat
                                || discard.Tile.Element0 != lastDiscard.Element0)
                            {
                                throw new OutOfSyncException(
                                    $"Previously discarded tile {lastDiscard}, but received" +
                                    $"discard event {discard}");
                            }

                            // Clear local tracking for discarded tile now that the server has
                            // caught up.
                            _lastDiscard = null;
                        }
                        else if (_localState.TryDiscardTile(discard.Seat, discard.Tile))
                        {
                            // Perform the discard action locally.
                            var hand = _hands[(int)discard.Seat];
                            hand.MoveToDiscard(discard.Tile);
                        }
                        else
                        {
                            throw new OutOfSyncException($"Could not apply discard event locally: {discard}");
                        }

                        // TODO: Reconcile our local state with the updated server state to
                        // verify that the two are in sync.
                    }
                    break;

                    case MatchEvent.MatchEnded _:
                    {
                        matchEnded = true;
                    }
                    break;
                }
            }

            // Display that the match ended UI and wait for the player to hit the exit button.
            _matchEndedDisplayRoot.SetActive(true);
            await _exitButton.OnClickAsync(_cancellation.Token);

            // TODO: Leave the match. But, like, in a cool way.
            throw new NotImplementedException("Return to the home screen");

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

        private async UniTask DiscardTile()
        {
            var hand = _hands[(int)_seat];
            var id = await hand.OnClickTileAsync(_cancellation.Token);

            // Attempt to discard the tile. If the operation fails, ignore the click event.
            if (!_localState.TryDiscardTile(hand.Seat, id))
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
            var request = _serverState.RequestDiscardTile(hand.Seat, id);
            _socket.SendString(request);
        }

        #region Unity Lifecycle Methods

        private void Awake()
        {
            // Validate that the `PlayerHand` object for each seat is correctly configured.
            foreach (var seat in EnumUtils.GetValues<Wind>())
            {
                var hand = _hands[(int)seat];
                Debug.Assert(
                    seat == hand.Seat,
                    $"{nameof(PlayerHand)} setup is incorrect, hand at seat {seat}" +
                    $"configured for seat {hand.Seat}");
            }
        }

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
