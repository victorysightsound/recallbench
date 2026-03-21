import vampire from './object.js';
import { addPrefix } from '../../functions/addPrefix.js';

export default ({ addBase, prefix = '' }) => {
  const prefixedvampire = addPrefix(vampire, prefix);
  addBase({ ...prefixedvampire });
};
