using Synapse.Utils;
using UnityEngine;

namespace Synapse.Mahjong
{
    /// <summary>
    /// Main controller for the mahjong gameplay.
    /// </summary>
    public class GameplayController : MonoBehaviour
    {
        [SerializeField] private Transform _boardObject = null;

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
