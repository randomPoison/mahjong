using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading;
using Synapse.Utils;
using UniRx.Async;
using UnityEngine;
using UnityEngine.AddressableAssets;

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

        #endregion

        #region Private Fields

        private WebSocket _socket;
        private ClientState _client;
        private MatchState _state;

        // Cached prefabs for the different tiles. These are populated during startup
        // based on the asset references configured above.
        private GameObject[] _bambooPrefabs = new GameObject[9];
        private GameObject[] _circlePrefabs = new GameObject[9];
        private GameObject[] _characterPrefabs = new GameObject[9];
        private GameObject[] _dragonPrefabs = new GameObject[3];
        private GameObject[] _windPrefabs = new GameObject[4];

        #endregion

        public async void Init(ClientState client, WebSocket socket)
        {
            _client = client;
            _socket = socket;

            // Request that the server start a new match, and load our tile prefabs in
            // the background. Wait for both of these operations to complete before
            // attempting to instantiate any tiles.
            await UniTask.WhenAll(
                RequestStartMatch(),
                LoadTilePrefabs());

            // Register input events from the player's hand.
            //
            // TODO: Have the server data specify which player is controlled by this
            // client, rather than hard coding it to always control the east seat.
            var playerHand = _hands[(int)Wind.East];
            playerHand.TileClicked += OnTileClicked;

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
                var tiles = _state.GetPlayerHand(seat);

                foreach (var tile in tiles)
                {
                    hand.AddToHand(InstantiateTile(tile));
                }

                if (_state.PlayerHasCurrentDraw(seat))
                {
                    var currentDraw = _state.GetCurrentDraw(seat);
                    hand.DrawTile(InstantiateTile(currentDraw));
                }
            }
        }

        private void OnDestroy()
        {
            _state?.Dispose();
            _state = null;

            // TODO: Unload any tile assets that we loaded? This *might* require some
            // logic to only attempt to unload assets that were successfully loaded in
            // the case where not all tile assets loaded before we leave the match.
        }

        /// <summary>
        /// Requests that the server start a new match.
        /// </summary>
        ///
        /// <returns>
        /// A task that resolves once the match has been successfully created.
        /// </returns>
        private async UniTask RequestStartMatch(CancellationToken cancellation = default)
        {
            // Request that the server start a match.
            var request = _client.CreateStartMatchRequest();
            _socket.SendString(request);
            var responseJson = await _socket.RecvStringAsync(cancellation);
            Debug.Log(responseJson, this);

            // TODO: Add some kind of error handling around failure. Probably not doable
            // until we can return more structured data from Rust functions.
            _state = _client.HandleStartMatchResponse(responseJson);
            Debug.Log($"Started match, ID: {_state.Id()}", this);
        }

        private void OnTileClicked(TileId id)
        {
            // TODO: Request from the server that the selected tile be discarded.
            throw new NotImplementedException();
        }

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
