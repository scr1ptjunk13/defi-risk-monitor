import { ExternalLinkIcon, InfoIcon } from "./Icons";
import { COMMISSION_RECIPIENT } from "../constants";
import type { FC } from "react";

const Footer: FC = () => {
  return (
    <footer className="bg-gray-900 border-t border-gray-700">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        <div className="grid grid-cols-1 md:grid-cols-3 gap-8">
          {/* About */}
          <div>
            <h3 className="text-lg font-semibold text-white mb-4">
              Uniswap Liquidity Creator
            </h3>
            <p className="text-gray-400 text-sm mb-4">
              Create liquidity pools for your tokens on Uniswap V3 with ease.
              Support for custom tokens and popular pairs on Polygon network.
            </p>
            <div className="flex items-center space-x-2 text-xs text-gray-500">
              <InfoIcon className="w-4 h-4" />
              <span>Built for Polygon Network</span>
            </div>
          </div>

          {/* Quick Links */}
          <div>
            <h3 className="text-lg font-semibold text-white mb-4">Quick Links</h3>
            <div className="space-y-2">
              <a
                href="https://docs.uniswap.org/"
                target="_blank"
                rel="noopener noreferrer"
                className="flex items-center space-x-2 text-gray-400 hover:text-white transition-colors text-sm"
              >
                <span>Uniswap Documentation</span>
                <ExternalLinkIcon className="w-3 h-3" />
              </a>
              <a
                href="https://app.uniswap.org/#/pools"
                target="_blank"
                rel="noopener noreferrer"
                className="flex items-center space-x-2 text-gray-400 hover:text-white transition-colors text-sm"
              >
                <span>Manage Your Pools</span>
                <ExternalLinkIcon className="w-3 h-3" />
              </a>
              <a
                href="https://polygonscan.com"
                target="_blank"
                rel="noopener noreferrer"
                className="flex items-center space-x-2 text-gray-400 hover:text-white transition-colors text-sm"
              >
                <span>Polygon Explorer</span>
                <ExternalLinkIcon className="w-3 h-3" />
              </a>
            </div>
          </div>

          {/* Commission Info */}
          <div>
            <h3 className="text-lg font-semibold text-white mb-4">
              Service Information
            </h3>
            <div className="space-y-3">
              <div className="text-sm">
                <p className="text-gray-300 font-medium">Commission Fee</p>
                <p className="text-gray-400">0.001 POL per transaction</p>
              </div>
              <div className="text-sm">
                <p className="text-gray-300 font-medium">Commission Wallet</p>
                <p className="text-gray-400 break-all font-mono text-xs">
                  {COMMISSION_RECIPIENT}
                </p>
              </div>
              <div className="text-xs text-gray-500">
                Commission helps maintain and improve the platform
              </div>
            </div>
          </div>
        </div>

        {/* Bottom Bar */}
        <div className="mt-8 pt-8 border-t border-gray-700">
          <div className="flex flex-col sm:flex-row items-center justify-between">
            <div className="text-sm text-gray-400">
              © 2025 Uniswap Liquidity Creator. Built with ❤️ for DeFi.
            </div>
            <div className="flex items-center space-x-4 mt-4 sm:mt-0">
              <div className="text-xs text-gray-500">
                Powered by Uniswap V3 Protocol
              </div>
              <div className="w-2 h-2 bg-green-500 rounded-full animate-pulse"></div>
            </div>
          </div>
        </div>
      </div>
    </footer>
  );
};

export default Footer;
