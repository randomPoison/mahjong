namespace Synapse.Mahjong
{
    /// <summary>
    /// Indicates the next screen to transition to when leaving a screen.
    /// </summary>
    public enum NextScreen
    {
        /// <summary>
        /// Return to the home screen.
        /// </summary>
        Home,

        /// <summary>
        /// Go to the match screen and begin a match.
        /// </summary>
        Match,
    }
}
