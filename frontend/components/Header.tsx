import { useState, type FC } from "react";
import { ConnectButton } from "@rainbow-me/rainbowkit";
import { WalletIcon, ExternalLinkIcon } from "./Icons";

const Header: FC = () => {
  const [isMobileMenuOpen, setIsMobileMenuOpen] = useState<boolean>(false);

  const toggleMobileMenu = (): void => {
    setIsMobileMenuOpen((prev) => !prev);
  };

  return (
    <header className="bg-gray-900 border-b border-gray-700">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div className="flex items-center justify-between h-16">
          {/* Logo - Responsive sizing */}
          <div className="flex items-center space-x-2 sm:space-x-3 flex-shrink-0">
            <div className="w-6 h-6 sm:w-8 sm:h-8 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg flex items-center justify-center">
              <WalletIcon className="w-3 h-3 sm:w-5 sm:h-5 text-white" />
            </div>
            <div className="min-w-0">
              <h1 className=" sm:hidden text-sm sm:text-lg md:text-xl font-bold text-white truncate">
                Liquidity Creator
              </h1>
              <h1 className="hidden sm:block text-sm sm:text-lg md:text-xl font-bold text-white truncate">
                Uniswap Liquidity Creator
              </h1>
              <p className="text-xs text-gray-400 hidden xs:block">Polygon Network</p>
            </div>
          </div>

          {/* Desktop Navigation */}
          <div className="hidden lg:flex items-center space-x-6">
            <a
              href="https://app.uniswap.org/#/pools"
              target="_blank"
              rel="noopener noreferrer"
              className="text-gray-300 hover:text-white transition-colors flex items-center space-x-1"
            >
              <span>View on Uniswap</span>
              <ExternalLinkIcon className="w-4 h-4" />
            </a>
            <a
              href="https://polygonscan.com"
              target="_blank"
              rel="noopener noreferrer"
              className="text-gray-300 hover:text-white transition-colors flex items-center space-x-1"
            >
              <span>PolygonScan</span>
              <ExternalLinkIcon className="w-4 h-4" />
            </a>
          </div>

          {/* Right side - Connect Button + Mobile Menu Toggle */}
          <div className="flex items-center space-x-2 sm:space-x-4">
            {/* Connect Wallet Button */}
            <div className="flex items-center">
              <ConnectButton />
            </div>
            {/* Mobile Menu Button */}
            <button
              onClick={toggleMobileMenu}
              className="lg:hidden p-2 rounded-md text-gray-400 hover:text-white hover:bg-gray-800 focus:outline-none focus:ring-2 focus:ring-inset focus:ring-white"
              aria-expanded={isMobileMenuOpen}
              aria-label="Toggle navigation menu"
            >
              <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                {isMobileMenuOpen ? (
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M6 18L18 6M6 6l12 12"
                  />
                ) : (
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M4 6h16M4 12h16M4 18h16"
                  />
                )}
              </svg>
            </button>
          </div>
        </div>

        {/* Mobile Navigation Menu */}
        {isMobileMenuOpen && (
          <div className="lg:hidden border-t border-gray-700">
            <div className="px-2 pt-2 pb-3 space-y-1">
              <a
                href="https://app.uniswap.org/#/pools"
                target="_blank"
                rel="noopener noreferrer"
                className="text-gray-300 hover:text-white hover:bg-gray-800 block px-3 py-2 rounded-md text-base font-medium transition-colors flex items-center justify-between"
                onClick={() => setIsMobileMenuOpen(false)}
              >
                <span>View on Uniswap</span>
                <ExternalLinkIcon className="w-4 h-4 ml-2" />
              </a>
              <a
                href="https://polygonscan.com"
                target="_blank"
                rel="noopener noreferrer"
                className="text-gray-300 hover:text-white hover:bg-gray-800 block px-3 py-2 rounded-md text-base font-medium transition-colors flex items-center justify-between"
                onClick={() => setIsMobileMenuOpen(false)}
              >
                <span>PolygonScan</span>
                <ExternalLinkIcon className="w-4 h-4 ml-2" />
              </a>
            </div>
          </div>
        )}
      </div>
    </header>
  );
};

export default Header;
