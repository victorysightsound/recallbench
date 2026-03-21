import midnight from './object.js';
import { addPrefix } from '../../functions/addPrefix.js';

export default ({ addBase, prefix = '' }) => {
  const prefixedmidnight = addPrefix(midnight, prefix);
  addBase({ ...prefixedmidnight });
};
