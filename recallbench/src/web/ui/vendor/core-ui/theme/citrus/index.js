import citrus from './object.js';
import { addPrefix } from '../../functions/addPrefix.js';

export default ({ addBase, prefix = '' }) => {
  const prefixedcitrus = addPrefix(citrus, prefix);
  addBase({ ...prefixedcitrus });
};
