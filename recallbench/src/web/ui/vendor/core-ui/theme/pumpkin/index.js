import pumpkin from './object.js';
import { addPrefix } from '../../functions/addPrefix.js';

export default ({ addBase, prefix = '' }) => {
  const prefixedpumpkin = addPrefix(pumpkin, prefix);
  addBase({ ...prefixedpumpkin });
};
