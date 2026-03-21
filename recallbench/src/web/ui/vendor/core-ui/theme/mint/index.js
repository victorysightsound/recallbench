import mint from './object.js';
import { addPrefix } from '../../functions/addPrefix.js';

export default ({ addBase, prefix = '' }) => {
  const prefixedmint = addPrefix(mint, prefix);
  addBase({ ...prefixedmint });
};
