import mocha from './object.js';
import { addPrefix } from '../../functions/addPrefix.js';

export default ({ addBase, prefix = '' }) => {
  const prefixedmocha = addPrefix(mocha, prefix);
  addBase({ ...prefixedmocha });
};
