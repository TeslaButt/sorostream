'use client';

import { useEffect, useState } from 'react';
import { isAllowed, setAllowed, requestAccess, getUserInfo } from '@stellar/freighter-api';

export default function WalletButton() {
  const [pubKey, setPubKey] = useState<string | null>(null);

  useEffect(() => {
    // Check if the user has previously approved the site
    const checkConnection = async () => {
      if (await isAllowed()) {
        const userInfo = await getUserInfo();
        if (userInfo.publicKey) {
          setPubKey(userInfo.publicKey);
        }
      }
    };
    checkConnection();
  }, []);

  const handleConnect = async () => {
    try {
      await setAllowed(); // prompts Freighter permissions
      const res = await requestAccess(); // requests auth
      if (res) {
        setPubKey(res);
      }
    } catch (e) {
      console.error('Connection failed', e);
    }
  };

  const shortenString = (str: string) => {
    if (!str) return '';
    return `${str.slice(0, 4)}...${str.slice(-4)}`;
  };

  return (
    <button
      onClick={pubKey ? undefined : handleConnect}
      className={`px-6 py-3 rounded-xl font-semibold transition-all duration-200 ${
        pubKey
          ? 'bg-sky-500/10 text-sky-400 border border-sky-500/20 cursor-default'
          : 'bg-gradient-to-r from-sky-500 to-blue-600 text-white hover:from-sky-400 hover:to-blue-500 shadow-lg shadow-sky-500/25'
      }`}
    >
      {pubKey ? `Connected: ${shortenString(pubKey)}` : 'Connect Freighter'}
    </button>
  );
}
