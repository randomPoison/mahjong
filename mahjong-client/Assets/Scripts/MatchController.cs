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

            foreach (var seat in EnumUtils.GetValues<Wind>())
            {
                var tiles = _state.GetPlayerHand(seat);
                Debug.Log($"Tiles @ {seat}: {tiles.Count}");

                foreach (var (index, tile) in tiles.Enumerate())
                {
                    switch (tile)
                    {
                        case Tile.Simple simple:
                            Debug.Log($"Tile @ {seat} #{index}: {simple.Element0.Number} of {simple.Element0.Suit}");
                            break;

                        case Tile.Dragon dragon:
                            Debug.Log($"Tile @ {seat} #{index}: {dragon.Element0} Dragon");
                            break;

                        case Tile.Wind wind:
                            Debug.Log($"Tile @ {seat} #{index}: {wind.Element0} Wind");
                            break;
                    }
                }
            }
        }

        private void OnDestroy()
        {
            _state?.Dispose();
            _state = null;
        }
    }
}
