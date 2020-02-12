using TMPro;
using UnityEngine;

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

        private ClientState _state;

        /// <summary>
        /// Initializes the controller. Must be called immediately upon loading the home
        /// screen in order to correctly initialize the scene.
        /// </summary>
        /// <param name="state"></param>
        public void Init(ClientState state)
        {
            _state = state;

            _accountIdDisplay.text = _state.AccountId().ToString();
            _pointsDisplay.text = _state.Points().ToString();
        }
    }
}
