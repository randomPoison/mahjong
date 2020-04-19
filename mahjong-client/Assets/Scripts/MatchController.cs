using System;
using Synapse.Utils;
using UnityEngine;
using UnityEngine.AddressableAssets;

namespace Synapse.Mahjong
{
    /// <summary>
    /// Main controller for the mahjong gameplay.
    /// </summary>
    public class MatchController : MonoBehaviour
    {
        // TODO: Figure out a better way to track tile dimensions. This should likely be
        // tracked along with the tile asset set, once we move the tile set to a custom
        // asset.
        private const float TileWidth = 0.026f;

        private const float LeftSide = TileWidth * TilesInAHand * -0.5f;
        private const int TilesInAHand = 13;

        [SerializeField]
        [Tooltip(
            "The root object for each player's hand. Tiles in each players' hand will " +
            "be made children of these objects.")]
        private Transform[] _handRoots = default;

        // TODO: Move the tile asset configuration into a scriptable object. While we
        // only have one set of tile assets we can get away with baking it directly into
        // the controller, but this setup won't scale.
        [Header("Tile Assets")]
        [SerializeField] private AssetReference[] _bambooTiles = default;
        [SerializeField] private AssetReference[] _circleTiles = default;
        [SerializeField] private AssetReference[] _characterTiles = default;
        [SerializeField] private AssetReference[] _dragonTiles = default;
        [SerializeField] private AssetReference[] _windTiles = default;

        private WebSocket _socket;
        private ClientState _client;
        private Match _state;

        public async void Init(ClientState client, WebSocket socket)
        {
            _client = client;
            _socket = socket;

            // Request that the server start a match.
            var request = client.CreateStartMatchRequest();
            _socket.SendString(request);
            var responseJson = await socket.RecvStringAsync();
            Debug.Log(responseJson, this);

            // TODO: Add some kind of error handling around failure. Probably not doable
            // until we can return more structured data from Rust functions.
            _state = _client.HandleStartMatchResponse(responseJson);
            Debug.Log($"Started match, ID: {_state.Id()}", this);

            // Once we have the match data, instantiate the tiles for each player's
            // starting hand.

            foreach (var seat in EnumUtils.GetValues<Wind>())
            {
                var handRoot = _handRoots[(int)seat];
                var tiles = _state.GetPlayerHand(seat);

                foreach (var (index, tile) in tiles.Enumerate())
                {
                    // Instantiate the prefab for the tile.
                    //
                    // TODO: Load the tile assets ahead of time. This setup is going to
                    // be unnecessarily slow because we load the tiles one at a time. We
                    // should preload all tiles from the set at startup, since we know
                    // we're going to need them all.
                    var asset = GetTileAsset(tile);
                    var tileObject = await Addressables.InstantiateAsync(asset).Task;

                    // Make the tile a child of the root object for the player's hand,
                    // and position it horizontally.
                    tileObject.transform.SetParent(handRoot, false);
                    tileObject.transform.localPosition = new Vector3(
                        LeftSide + TileWidth * index,
                        0f,
                        0f);
                }
            }
        }

        private void OnDestroy()
        {
            _state?.Dispose();
            _state = null;
        }

        private AssetReference GetTileAsset(ITile tile)
        {
            switch (tile)
            {
                case Tile.Simple simple:
                    // Tile numbers start at 1, so we need to subtract 1 to get the
                    // index corresponding to the tile's numeric value.
                    int tileIndex = simple.Element0.Number - 1;

                    switch (simple.Element0.Suit)
                    {
                        case Suit.Bamboo: return _bambooTiles[tileIndex];
                        case Suit.Characters: return _characterTiles[tileIndex];
                        case Suit.Coins: return _circleTiles[tileIndex];

                        default: throw new ArgumentException(
                            $"Invalid suit {simple.Element0.Suit}");
                    }

                case Tile.Dragon dragon:
                    return _dragonTiles[(int)dragon.Element0];

                case Tile.Wind wind:
                    return _windTiles[(int)wind.Element0];

                default: throw new ArgumentException($"Invalid tile kind {tile}");
            }
        }
    }
}
