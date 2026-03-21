import pearl from './object.js';
import { addPrefix } from '../../functions/addPrefix.js';

export default ({ addBase, prefix = '' }) => {
  const prefixedpearl = addPrefix(pearl, prefix);
  addBase({ ...prefixedpearl });
};
