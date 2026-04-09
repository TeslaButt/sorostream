'use client';

import { useState } from 'react';

export default function CreateStreamForm() {
  const [recipient, setRecipient] = useState('');
  const [tokenAddress, setTokenAddress] = useState('');
  const [amount, setAmount] = useState('');
  const [durationDays, setDurationDays] = useState('30');
  const [status, setStatus] = useState<{ type: 'idle' | 'success' | 'error', msg: string }>({ type: 'idle', msg: '' });

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setStatus({ type: 'idle', msg: '' });

    if (!recipient || !tokenAddress || !amount || !durationDays) {
      setStatus({ type: 'error', msg: 'Please fill out all fields.' });
      return;
    }

    if (Number(amount) <= 0) {
      setStatus({ type: 'error', msg: 'Amount must be positive.' });
      return;
    }

    // Since the smart contract is not deployed yet, we mock the submission
    // and console log the expected parameters to demonstrate it works.
    setStatus({ type: 'idle', msg: 'Building transaction...' });

    console.log('--- SoroStream Transaction Builder ---');
    console.log('Action: create_stream');
    console.log('Recipient:', recipient);
    console.log('Token (SAC):', tokenAddress);
    console.log('Total Amount:', amount);
    
    // Calculate unix timestamps for the smart contract parameters
    const start_time = Math.floor(Date.now() / 1000) + 60; // start 1 min from now
    const end_time = start_time + (Number(durationDays) * 24 * 60 * 60);

    console.log('Start Time (unix):', start_time);
    console.log('End Time (unix):', end_time);
    console.log('--- End Builder ---');

    setTimeout(() => {
      setStatus({ type: 'success', msg: 'Transaction payload built! See console log for details.' });
    }, 800);
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-4">
      <div className="space-y-1">
        <label className="text-sm text-gray-300 font-medium ml-1">Recipient Address</label>
        <input
          type="text"
          value={recipient}
          onChange={(e) => setRecipient(e.target.value)}
          placeholder="G..."
          className="w-full bg-white/5 border border-white/10 rounded-xl px-4 py-3 text-white placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-sky-500/50"
        />
      </div>

      <div className="space-y-1">
        <label className="text-sm text-gray-300 font-medium ml-1">Token SAC Address</label>
        <input
          type="text"
          value={tokenAddress}
          onChange={(e) => setTokenAddress(e.target.value)}
          placeholder="C..."
          className="w-full bg-white/5 border border-white/10 rounded-xl px-4 py-3 text-white placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-sky-500/50"
        />
      </div>

      <div className="grid grid-cols-2 gap-4">
        <div className="space-y-1">
          <label className="text-sm text-gray-300 font-medium ml-1">Amount</label>
          <input
            type="number"
            value={amount}
            onChange={(e) => setAmount(e.target.value)}
            placeholder="0.00"
            min="0"
            step="0.0000001"
            className="w-full bg-white/5 border border-white/10 rounded-xl px-4 py-3 text-white placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-sky-500/50"
          />
        </div>

        <div className="space-y-1">
          <label className="text-sm text-gray-300 font-medium ml-1">Duration (Days)</label>
          <input
            type="number"
            value={durationDays}
            onChange={(e) => setDurationDays(e.target.value)}
            min="1"
            className="w-full bg-white/5 border border-white/10 rounded-xl px-4 py-3 text-white focus:outline-none focus:ring-2 focus:ring-sky-500/50"
          />
        </div>
      </div>

      {status.msg && (
        <div className={`text-sm px-4 py-2 rounded-lg border ${
          status.type === 'error' ? 'bg-red-500/10 border-red-500/20 text-red-400' : 
          status.type === 'success' ? 'bg-green-500/10 border-green-500/20 text-green-400' :
          'text-gray-400'
        }`}>
          {status.msg}
        </div>
      )}

      <button
        type="submit"
        className="w-full mt-4 bg-gradient-to-r from-sky-500 to-blue-600 hover:from-sky-400 hover:to-blue-500 text-white font-bold py-3 px-4 rounded-xl shadow-lg shadow-sky-500/25 transition-all"
      >
        Create Stream
      </button>
    </form>
  );
}
