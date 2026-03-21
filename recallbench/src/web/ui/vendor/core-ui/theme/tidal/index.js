import tidal from './object.js';
import { addPrefix } from '../../functions/addPrefix.js';

export default ({ addBase, prefix = '' }) => {
  const prefixedtidal = addPrefix(tidal, prefix);
  addBase({ ...prefixedtidal });
};
