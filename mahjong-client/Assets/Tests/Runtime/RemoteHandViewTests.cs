using NUnit.Framework;
using Synapse.Mahjong.Match;
using System.Collections;
using UnityEditor;
using UnityEngine;
using UnityEngine.TestTools;

namespace Synapse.Mahjong.Tests
{
    public class RemoteHandViewTests
    {
        private RemoteHandView _prefab;
        private TileView _tilePrefab;

        private RemoteHandView _instance;
        private Transform _tileRoot;
        private Transform _drawTileAnchor;
        private Transform _discardRoot;
        private Transform _meldRoot;

        #region Setup and Tear Down

        [OneTimeSetUp]
        public void OneTimeSetup()
        {
            _prefab = AssetDatabase.LoadAssetAtPath<RemoteHandView>(
                "Assets/Prefabs/Match/Remote Hand View.prefab");
            _tilePrefab = AssetDatabase.LoadAssetAtPath<TileView>(
                "Assets/Prefabs/Tiles/Circle_1.prefab");
        }

        [SetUp]
        public void Setup()
        {
            _instance = Object.Instantiate(_prefab);
            _tileRoot = _instance.transform.Find("Tile Root");
            _drawTileAnchor = _instance.transform.Find("Draw Tile Anchor");
            _discardRoot = _instance.transform.Find("Discard Root");
            _meldRoot = _instance.transform.Find("Meld Root");

            // Populate the hand before each test so that we're testing a valid setup.
            _instance.FillWithDummyTiles(_tilePrefab.gameObject);
        }

        [TearDown]
        public void TearDown()
        {
            Object.Destroy(_instance);
        }

        #endregion

        [Test]
        public void InitialHandHasCorrectTiles()
        {
            Assert.AreEqual(13, _tileRoot.childCount, "Starting hand should have 13 tiles once setup");
            Assert.AreEqual(0, _drawTileAnchor.childCount, "Starting hand should not have a draw tile");
            Assert.AreEqual(0, _discardRoot.childCount, "Starting hand should not have discards");
            Assert.AreEqual(0, _meldRoot.childCount, "Starting hand should not have melds");
        }

        [Test]
        public void CallTilesRemovedFromHand()
        {
            _instance.CallTile(
                Object.Instantiate(_tilePrefab),
                new Call.Chii(new TileId(0), new TileId(1)),
                id => Object.Instantiate(_tilePrefab));

            Assert.AreEqual(10, _tileRoot.childCount, "Incorrect number of tiles remaining in hand after call");
            Assert.AreEqual(3, _meldRoot.childCount, "Incorrect number of meld tiles after call");
        }
    }
}
