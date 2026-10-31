import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import './styles/globals.css';
import './styles/product.css';
import { syncAppearance } from './store/appearance';

const stopAppearanceSync = syncAppearance();
if (import.meta.hot) import.meta.hot.dispose(stopAppearanceSync);

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
