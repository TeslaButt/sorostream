import type { Metadata } from 'next';
import WalletButton from '@/components/WalletButton';
import CreateStreamForm from '@/components/CreateStreamForm';

export const metadata: Metadata = {
  title: 'SoroStream — Real-time Token Streaming on Stellar Soroban',
  description:
    'Create and claim real-time token streams on Stellar. Lock tokens that vest continuously — no cliffs, no trust required.',
};

export default function HomePage() {
  return (
    <main className="min-h-screen flex flex-col items-center justify-center px-4 py-20">
      {/* ── Hero ── */}
      <section className="text-center max-w-3xl mx-auto space-y-6">
        <div className="inline-flex items-center gap-2 px-4 py-1.5 rounded-full text-sm font-medium bg-sky-500/10 text-sky-400 border border-sky-500/20 mb-4">
          🚀 Built on Stellar Soroban
        </div>

        <h1 className="text-5xl md:text-6xl font-extrabold tracking-tight gradient-text">
          Token Streaming,<br />Trustlessly.
        </h1>

        <p className="text-lg text-gray-400 max-w-xl mx-auto leading-relaxed">
          SoroStream lets you create real-time token vesting streams on Stellar.
          Recipients claim tokens as they vest — second by second, no intermediary required.
        </p>

        <div className="flex flex-wrap gap-4 justify-center pt-2">
          <WalletButton />
          <a
            href="https://github.com/YOUR_ORG/sorostream"
            id="link-view-contract"
            target="_blank"
            rel="noopener noreferrer"
            className="px-6 py-3 rounded-xl font-semibold text-gray-300 border border-white/10 hover:bg-white/5 transition-all duration-200"
          >
            View on GitHub →
          </a>
        </div>
      </section>

      {/* ── Feature Cards ── */}
      <section className="mt-24 grid grid-cols-1 md:grid-cols-3 gap-6 max-w-5xl w-full">
        {[
          {
            icon: '⏱',
            title: 'Real-time Streaming',
            description:
              'Tokens vest continuously. Recipients can claim their pro-rata share at any ledger — no waiting for cliff dates.',
          },
          {
            icon: '🔒',
            title: 'Non-custodial',
            description:
              'Funds are locked in a Soroban smart contract, not held by SoroStream. No keys, no trust.',
          },
          {
            icon: '🧩',
            title: 'Open Source',
            description:
              'Fully open-source and contributor-friendly. Funded by Drips Wave grants. Fork it, build on it.',
          },
        ].map((card) => (
          <div key={card.title} className="glass-card p-6 space-y-3">
            <span className="text-3xl">{card.icon}</span>
            <h2 className="text-lg font-semibold text-white">{card.title}</h2>
            <p className="text-sm text-gray-400 leading-relaxed">{card.description}</p>
          </div>
        ))}
      </section>

      {/* ── Stream UI ── */}
      <section className="mt-24 w-full max-w-xl">
        <div className="glass-card p-8 bg-black/20 backdrop-blur-xl border border-white/10 shadow-2xl rounded-3xl">
          <div className="mb-6">
            <h2 className="text-2xl font-bold text-white mb-2">Create a Stream</h2>
            <p className="text-sm text-gray-400">
              Lock tokens into the SoroStream protocol. They will stream continuously to the recipient.
            </p>
          </div>
          <CreateStreamForm />
        </div>
      </section>
    </main>
  );
}
