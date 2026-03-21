import starlight from './object.js';
import { addPrefix } from '../../functions/addPrefix.js';

export default ({ addBase, prefix = '' }) => {
  const prefixedstarlight = addPrefix(starlight, prefix);
  addBase({ ...prefixedstarlight });
};
