import vintage from './object.js';
import { addPrefix } from '../../functions/addPrefix.js';

export default ({ addBase, prefix = '' }) => {
  const prefixedvintage = addPrefix(vintage, prefix);
  addBase({ ...prefixedvintage });
};
