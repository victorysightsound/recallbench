import evergreen from './object.js';
import { addPrefix } from '../../functions/addPrefix.js';

export default ({ addBase, prefix = '' }) => {
  const prefixedevergreen = addPrefix(evergreen, prefix);
  addBase({ ...prefixedevergreen });
};
