import electric from './object.js';
import { addPrefix } from '../../functions/addPrefix.js';

export default ({ addBase, prefix = '' }) => {
  const prefixedelectric = addPrefix(electric, prefix);
  addBase({ ...prefixedelectric });
};
