import print from './object.js';
import { addPrefix } from '../../functions/addPrefix.js';

export default ({ addBase, prefix = '' }) => {
  const prefixedprint = addPrefix(print, prefix);
  addBase({ ...prefixedprint });
};
