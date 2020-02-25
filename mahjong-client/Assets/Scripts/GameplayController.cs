using UnityEngine;

namespace Synapse.Mahjong
{
    /// <summary>
    /// Main controller for the mahjong gameplay.
    /// </summary>
    public class GameplayController : MonoBehaviour
    {
        [SerializeField] private Transform _boardObject = null;

        private ClientState _client;

        public async void Init(ClientState client)
        {
            _client = client;


        }
    }
}
