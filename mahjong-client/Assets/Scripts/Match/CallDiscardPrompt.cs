using Synapse.Mahjong.Match;
using Synapse.Utils;
using System.Collections.Generic;
using System.Threading;
using UniRx.Async;
using UnityEngine;
using UnityEngine.UI;

namespace Synapse.Mahjong
{
    /// <summary>
    /// The prompt shown the player when they have the option to call a discarded tile.
    /// </summary>
    public class CallDiscardPrompt : MonoBehaviour
    {
        [SerializeField] private RectTransform _callsRoot = default;
        [SerializeField] private Button _passButton = default;

        [SerializeField] private CallView _callPrefab = default;

        public async UniTask<ICall> MakeCall(
            List<ICall> calls,
            CancellationToken cancellation = default)
        {
            var linkedCancellation = CancellationTokenSource.CreateLinkedTokenSource(cancellation);
            try
            {
                var callSelections = new List<UniTask<ICall>>();
                foreach (var call in calls)
                {
                    var callView = Instantiate(_callPrefab, _callsRoot, false);
                    callView.Init(call);
                    callSelections.Add(callView.OnClickAsync(linkedCancellation.Token));
                }

                callSelections.Add(OnPassAsync());

                // Wait for any of the buttons to be selected.
                //
                // NOTE: We must re-await the task that finished in order to propagate
                // the exception if the task finished due to an exception.
                var (winIndex, selection) = await UniTask.WhenAny(callSelections.ToArray());
                await callSelections[winIndex];

                return selection;
            }
            finally
            {
                // Cancel the remaining tasks for the buttons that weren't pressed.
                linkedCancellation.Cancel();
                linkedCancellation.Dispose();

                // Destroy view objects for the calls.
                //
                // TODO: Pool the view objects instead of completely destroying them?
                _callsRoot.DestroyChildren();
            }

            // Helper function for handling when the player clicks the pass button.
            // Returns null to indicate that no call was made.
            async UniTask<ICall> OnPassAsync()
            {
                await _passButton.OnClickAsync(linkedCancellation.Token);
                return null;
            }
        }
    }
}
