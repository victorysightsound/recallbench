import depths from './object.js';
import { addPrefix } from '../../functions/addPrefix.js';

export default ({ addBase, prefix = '' }) => {
  const prefixeddepths = addPrefix(depths, prefix);
  addBase({ ...prefixeddepths });
};
