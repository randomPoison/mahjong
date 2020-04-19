using Synapse.Mahjong.Match;
using TMPro;
using UniRx.Async;
using UnityEngine;
using UnityEngine.SceneManagement;
using UnityEngine.UI;

namespace Synapse.Mahjong
{
    /// <summary>
    /// Main controller for the home screen. Displays account information and handles
    /// logic for the play button.
    /// </summary>
    public class HomeController : MonoBehaviour
    {
        [SerializeField] private TextMeshProUGUI _accountIdDisplay = null;
        [SerializeField] private TextMeshProUGUI _pointsDisplay = null;
        [SerializeField] private Button _playButton = null;

        private WebSocket _socket;
        private ClientState _state;
        private Scene _scene;

        /// <summary>
        /// Initializes the controller. Must be called immediately upon loading the home
        /// screen in order to correctly initialize the scene.
        /// </summary>
        ///
        /// <param name="state">
        /// The <see cref="ClientState"/> object for the current client.
        /// </param>
        public void Init(ClientState state, WebSocket socket)
        {
            _state = state;
            _socket = socket;
            _scene = SceneManager.GetSceneByName("Home");

            _accountIdDisplay.text = _state.AccountId().ToString();
            _pointsDisplay.text = _state.Points().ToString();

            _playButton.onClick.AddListener(LoadGameplayScene);
        }

        private async void LoadGameplayScene()
        {
            var unloadTask = SceneManager.UnloadSceneAsync(_scene);
            var loadTask = SceneManager.LoadSceneAsync("Gameplay", LoadSceneMode.Additive);
            await UniTask.WhenAll(unloadTask.ToUniTask(), loadTask.ToUniTask());

            // Initialize the scene controller.
            FindObjectOfType<MatchController>().Init(_state, _socket);
        }
    }
}
