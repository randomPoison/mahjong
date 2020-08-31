using NUnit.Framework;
using System.Collections;
using System.Collections.Generic;
using System.Threading;
using UnityEditor;
using UnityEngine;
using UnityEngine.TestTools;
using UnityEngine.UI;

namespace Synapse.Mahjong.Tests
{
    public class CallDiscardPromptTests
    {
        // Reference to the prefab. Loaded once on startup.
        private CallDiscardPrompt _prefab;

        // References for the prompt instance. These are setup per-test, with a fresh
        // instance of the prompt created for each test case.
        private CallDiscardPrompt _instance;
        private Transform _callsRoot;
        private Button _passButton;

        // NOTE: Use this cancellation token source for any async operations spawned in
        // the test. Cancellation is requested as part of tear down after each test case
        // in order to cancel any tasks that weren't finished as part of the test case.
        private CancellationTokenSource _cancellation;

        #region Setup and Tear Down

        [OneTimeSetUp]
        public void LoadPrefab()
        {
            _prefab = AssetDatabase.LoadAssetAtPath<CallDiscardPrompt>(
                "Assets/Prefabs/Match/Call Discard Prompt.prefab");
        }

        [SetUp]
        public void Setup()
        {
            _instance = Object.Instantiate(_prefab);
            _callsRoot = _instance.transform.Find("Calls");
            _passButton = _instance.transform.Find("Pass Button").GetComponent<Button>();

            _cancellation = new CancellationTokenSource();
        }

        [TearDown]
        public void DestroyInstance()
        {
            _cancellation.Cancel();
            _cancellation.Dispose();
            Object.Destroy(_instance);
        }

        #endregion

        // Test that the prompt always displays the correct number of options even after
        // being used multiple times. This is to catch an issue that can happen if we
        // don't clear the list of options after a call is made, resulting in there being
        // invalid options displayed for subsequent usages of the prompt.
        [UnityTest]
        public IEnumerator PromptDisplaysCorrectNumberOfOptionsOnRepeatedCalls()
        {
            for (var count = 0; count < 10; count += 1)
            {
                // Populate the prompt with a dummy call.
                _ = _instance.MakeCall(
                    new List<ICall> {
                        new Call.Chii(new TileId(0), new TileId(1)),
                        new Call.Pon(new TileId(0), new TileId(1)),
                    },
                    _cancellation.Token);

                // Wait a frame so that Unity can finish instantiating the relevant objects.
                yield return null;

                // Verify that there is 2 option for the call.
                Assert.AreEqual(
                    2,
                    _callsRoot.childCount,
                    $"Incorrect number of calls displayed on attempt {count}");

                // Invoke the pass button before re-populating the prompt, since it's
                // not valid to populate it a second time before a selection has been
                // made. We could also cancel the task to achieve the same effect, but
                // this is simpler.
                _passButton.onClick.Invoke();
                yield return null;
            }
        }

        [UnityTest]
        public IEnumerator TaskResolvesToNullWhenPassClicked()
        {
            // Populate the prompt with a dummy call.
            var selectionTask = _instance.MakeCall(
                new List<ICall> {
                    new Call.Chii(new TileId(0), new TileId(1)),
                    new Call.Pon(new TileId(0), new TileId(1)),
                },
                _cancellation.Token);

            // Wait a frame so that Unity can finish instantiating the relevant objects.
            yield return null;

            // Simulate that the button was clicked.
            _passButton.onClick.Invoke();

            // Verify that the task has resolved and that it returned `null` to indicate
            // that no call was selected.
            Assert.IsTrue(selectionTask.IsCompleted, "The task should resolve once a button is clicked");
            Assert.IsNull(selectionTask.Result, "The resulting task should be null when the player chooses to pass");
        }
    }
}
