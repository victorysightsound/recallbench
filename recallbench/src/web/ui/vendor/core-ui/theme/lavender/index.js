import lavender from './object.js';
import { addPrefix } from '../../functions/addPrefix.js';

export default ({ addBase, prefix = '' }) => {
  const prefixedlavender = addPrefix(lavender, prefix);
  addBase({ ...prefixedlavender });
};
