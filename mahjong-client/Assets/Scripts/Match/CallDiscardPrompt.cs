using System.Collections.Generic;
using System.Threading;
using UniRx.Async;
using UnityEngine;
using UnityEngine.UI;

namespace Synapse.Mahjong
{
    public class CallDiscardPrompt : MonoBehaviour
    {
        [SerializeField] private RectTransform _callsRoot = default;
        [SerializeField] private Button _passButton = default;

        public async UniTask<ICall> MakeCall(List<ICall> calls, CancellationToken cancellation)
        {
            // TODO: Display a list of calls to make.

            await _passButton.OnClickAsync(cancellation);

            return null;
        }
    }
}
