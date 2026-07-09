import { render } from 'solid-js/web';
import '@forge/tokens/fonts.css';
import '@forge/tokens/tokens.css';
import '@forge/tokens/base.css';
import '@forge/ui/styles.css';
import App from './App';

document.body.style.margin = '0';
render(() => <App />, document.getElementById('root'));
