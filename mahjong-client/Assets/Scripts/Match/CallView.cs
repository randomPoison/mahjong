using System.Threading;
using TMPro;
using UniRx.Async;
using UnityEngine;
using UnityEngine.UI;

namespace Synapse.Mahjong.Match
{
    /// <summary>
    /// The view for a single call presenting to a player when they have the option to
    /// call a discarded tile.
    /// </summary>
    ///
    /// <remarks>
    /// Used by <see cref="CallDiscardPrompt"/>.
    /// </remarks>
    public class CallView : MonoBehaviour
    {
        [SerializeField] private Button _button = default;
        [SerializeField] private TextMeshProUGUI _text = default;

        private ICall _model;

        public void Init(ICall model)
        {
            _model = model;
            _text.text = _model.ToString();
        }

        public async UniTask<ICall> OnClickAsync(CancellationToken cancellation = default)
        {
            await _button.OnClickAsync(cancellation);
            return _model;
        }
    }
}
