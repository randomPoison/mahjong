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
        }

        private void OnDestroy()
        {
            _state?.Dispose();
            _state = null;
        }
    }
}
