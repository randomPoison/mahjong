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

        /// <summary>
        /// Initializes the controller. Must be called immediately upon loading the home
        /// screen in order to correctly initialize the scene.
        /// </summary>
        ///
        /// <param name="state">
        /// The <see cref="ClientState"/> object for the current client.
        /// </param>
        public async UniTask<NextScreen> Run(ClientState state)
        {
            _accountIdDisplay.text = state.AccountId().ToString();
            _pointsDisplay.text = state.Points().ToString();

            // Wait for the player to hit the "Play" button since it's the only
            // interactive element in the scene.
            await _playButton.OnClickAsync();

            return NextScreen.Match;
        }
    }
}
