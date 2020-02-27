using System;
using System.Threading.Tasks;
using UniRx.Async;
using UnityEngine;
using UnityEngine.SceneManagement;

namespace Synapse.Mahjong
{
    /// <summary>
    /// Root controller for the game. Owns core game state and manages the high-level
    /// flow from scene to scene.
    /// </summary>
    public class StartupController : MonoBehaviour
    {
        private WebSocket _socket;
        private ClientState _state;

        private async void Start()
        {
            try
            {
                // Create the underlying state data for the client.
                _state = new ClientState();

                // TODO: Handle an exception being thrown as a result of the connection failing.
                // TODO: Make server address configurable.
                _socket = await WebSocket.ConnectAsync(new Uri("ws://localhost:3030/client"));

                // HACK: Due to a bug in WebSocketSharp, the first message that we'll receive
                // when waiting will be the initial ping sent by the server to trigger the
                // client connection.
                var pingHack = await _socket.RecvStringAsync();
                Debug.Assert(pingHack == "ping", $"Received unexpected first message: {pingHack}");

                Debug.Log("Established connection with server, beginning handshake");

                // TODO: Load any cached credentials and add them to the client state.

                // Perform handshake and initialization sequence with the server:
                //
                // * The client sends account ID and initial configuration data.
                // * Server sends current account data and any updated cache data.
                _socket.SendString(_state.CreateHandshakeRequest());
                if (!_state.HandleHandshakeResponse(await _socket.RecvStringAsync()))
                {
                    // TODO: Add better handling for the case where we failed to connect to
                    // the server. Obviously we need better error handling coming from the
                    // Rust side, be we should probably have some option for the player to
                    // re-attempt the connection.
                    Debug.LogError("Handshake with server failed :'(");
                    return;
                }

                Debug.Log($"Handshake completed, account ID: {_state.AccountId()}, points balance: {_state.Points()}");

                // Load the home screen once everything has been loaded.
                await SceneManager.LoadSceneAsync("Home", LoadSceneMode.Additive);

                // Once the home screen has loaded, initialize the home screen controller.
                var homeController = FindObjectOfType<HomeController>();
                homeController.Init(_state, _socket);

                // TODO: Wait for something to happen I guess?
            }
            catch (TaskCanceledException exception)
            {
                Debug.LogFormat($"Main task was canceled: {exception}");
            }
        }

        private void OnDestroy()
        {
            _socket?.Close();
            _socket = null;

            _state?.Dispose();
            _state = null;
        }
    }
}
