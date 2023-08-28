pragma ton-solidity >= 0.66.0;
pragma AbiHeader expire;
pragma AbiHeader time;
pragma AbiHeader pubkey;
import "https://raw.githubusercontent.com/tonlabs/debots/main/Debot.sol";
import "https://raw.githubusercontent.com/tonlabs/DeBot-IS-consortium/main/Terminal/Terminal.sol";

contract Debot1 is Debot {
    bytes _icon;
    //
    // DeBot mandatory functions
    //

    /// @notice Returns Metadata about DeBot.
    function getDebotInfo() public functionID(0xDEB) override view returns(
        string name, string version, string publisher, string caption, string author,
        address support, string hello, string language, string dabi, bytes icon
    ) {
        name = "Debot1";
        version = "0.1.1";
        publisher = "Ever Surf";
        caption = "Debot1_caption";
        author = "Ever Surf";
        support = address(0x606545c3b681489f2c217782e2da2399b0aed8640ccbcf9884f75648304dbc77);
        hello = "Debot1_hello";
        language = "en";
        dabi = m_debotAbi.get();
        icon = _icon;
    }

    function getRequiredInterfaces() public view override returns (uint256[] interfaces) {
        return [ Terminal.ID ];
    }
   
   function start() public override {
    Terminal.print(0, "Started");
   }
}