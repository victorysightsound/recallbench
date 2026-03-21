import royal from './object.js';
import { addPrefix } from '../../functions/addPrefix.js';

export default ({ addBase, prefix = '' }) => {
  const prefixedroyal = addPrefix(royal, prefix);
  addBase({ ...prefixedroyal });
};
