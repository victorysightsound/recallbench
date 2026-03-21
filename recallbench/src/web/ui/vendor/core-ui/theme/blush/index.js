import blush from './object.js';
import { addPrefix } from '../../functions/addPrefix.js';

export default ({ addBase, prefix = '' }) => {
  const prefixedblush = addPrefix(blush, prefix);
  addBase({ ...prefixedblush });
};
