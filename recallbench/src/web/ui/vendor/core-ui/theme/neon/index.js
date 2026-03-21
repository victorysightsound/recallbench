import neon from './object.js';
import { addPrefix } from '../../functions/addPrefix.js';

export default ({ addBase, prefix = '' }) => {
  const prefixedneon = addPrefix(neon, prefix);
  addBase({ ...prefixedneon });
};
