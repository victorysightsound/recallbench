import slate from './object.js';
import { addPrefix } from '../../functions/addPrefix.js';

export default ({ addBase, prefix = '' }) => {
  const prefixedslate = addPrefix(slate, prefix);
  addBase({ ...prefixedslate });
};
