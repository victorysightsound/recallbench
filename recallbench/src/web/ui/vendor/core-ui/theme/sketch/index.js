import sketch from './object.js';
import { addPrefix } from '../../functions/addPrefix.js';

export default ({ addBase, prefix = '' }) => {
  const prefixedsketch = addPrefix(sketch, prefix);
  addBase({ ...prefixedsketch });
};
